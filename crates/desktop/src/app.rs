use crate::{
    grid::{self, DataView},
    mcd::{self, DiagramView},
    sql::{self, SqlView},
    theme,
};
use egui::{RichText, Ui};
use kortex_application::SecretStore;
use kortex_domain::*;
use kortex_infrastructure::{
    storage::{self, NativeSecrets, Preferences},
    worker::{Operation, Payload, Request, Worker},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, VecDeque},
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

#[derive(Clone, PartialEq, Serialize, Deserialize)]
enum Pane {
    Explorer,
    Workspace,
    Inspector,
    Messages,
}
struct Connection {
    profile: Profile,
    password: String,
    worker: Worker,
    catalog: Catalog,
    ready: bool,
    catalog_pending: bool,
    last_schema: Instant,
    diagram: Diagram,
    history: LayoutHistory,
    error: Option<String>,
    failures: u32,
    retry_at: Option<Instant>,
}
enum DocumentKind {
    Data(Box<DataView>),
    Sql(SqlView),
    Diagram(DiagramView),
}
struct Document {
    id: u64,
    connection: String,
    title: String,
    kind: DocumentKind,
}
struct Detached {
    id: u64,
    connection: String,
    title: String,
    members: Vec<String>,
    view: DiagramView,
}
#[derive(Clone)]
enum Job {
    Catalog(String),
    Page(u64),
    Apply(u64),
}
enum Action {
    Connect(Profile, String),
    OpenTable(String, String),
    Diagram(String),
    Sql(String),
    Refresh(u64),
    Apply(u64),
    Execute(u64),
    Cancel(u64),
    Close(u64),
    Forget(String),
    Reconnect(String),
    SwitchDatabase(String, String),
}
struct ConnectionForm {
    id: String,
    name: String,
    sqlite: bool,
    path: String,
    host: String,
    port: u16,
    database: String,
    user: String,
    password: String,
    tls: bool,
    ca: String,
    remember: bool,
}
impl Default for ConnectionForm {
    fn default() -> Self {
        Self {
            id: new_id(),
            name: "PostgreSQL local".into(),
            sqlite: false,
            path: String::new(),
            host: "localhost".into(),
            port: 5432,
            database: "postgres".into(),
            user: "postgres".into(),
            password: String::new(),
            tls: true,
            ca: String::new(),
            remember: false,
        }
    }
}
impl ConnectionForm {
    fn from_profile(profile: &Profile) -> Self {
        let mut form = Self {
            id: profile.id.clone(),
            name: profile.name.clone(),
            remember: profile.remember_password,
            ..Self::default()
        };
        match &profile.engine {
            Engine::Sqlite { path } => {
                form.sqlite = true;
                form.path = path.clone();
            }
            Engine::Postgres {
                host,
                port,
                database,
                user,
                tls,
                ca_file,
            } => {
                form.host = host.clone();
                form.port = *port;
                form.database = database.clone();
                form.user = user.clone();
                form.tls = *tls;
                form.ca = ca_file.clone();
            }
        }
        form.password = NativeSecrets
            .get(&profile.id)
            .ok()
            .flatten()
            .unwrap_or_default();
        form
    }
    fn profile(&self) -> Profile {
        Profile {
            id: self.id.clone(),
            name: self.name.clone(),
            remember_password: self.remember,
            engine: if self.sqlite {
                Engine::Sqlite {
                    path: self.path.clone(),
                }
            } else {
                Engine::Postgres {
                    host: self.host.clone(),
                    port: self.port,
                    database: self.database.clone(),
                    user: self.user.clone(),
                    tls: self.tls,
                    ca_file: self.ca.clone(),
                }
            },
        }
    }
}
pub struct KortexApp {
    prefs: Preferences,
    path: Option<PathBuf>,
    tree: Option<egui_tiles::Tree<Pane>>,
    connections: BTreeMap<String, Connection>,
    documents: Vec<Document>,
    detached: Vec<Detached>,
    active: Option<u64>,
    selected: Option<(String, String)>,
    next_id: u64,
    jobs: BTreeMap<u64, Job>,
    actions: Vec<Action>,
    messages: VecDeque<String>,
    search: String,
    form: Option<ConnectionForm>,
    settings: bool,
    last_save: Instant,
    close_document: Option<u64>,
    close_app: bool,
    allow_close: bool,
    logo: Option<egui::TextureHandle>,
    navigation: Vec<(String, String)>,
    navigation_index: usize,
}
impl KortexApp {
    pub fn new(ctx: &egui::Context) -> Self {
        theme::apply(ctx);
        let path = storage::config_path().ok();
        let loaded = path
            .as_ref()
            .map(|p| storage::load(p))
            .unwrap_or_else(|| Ok(Preferences::default()));
        let (prefs, error) = match loaded {
            Ok(p) => (p, None),
            Err(e) => (Preferences::default(), Some(e.to_string())),
        };
        let mut app = Self::with_preferences(prefs);
        // A corrupt or newer configuration must never be overwritten by defaults.
        if error.is_none() {
            app.path = path;
        } else if let Some(error) = error {
            app.message(format!(
                "Configuration non chargée, sauvegarde désactivée : {error}"
            ));
        }
        if let Ok(image) = image::load_from_memory(include_bytes!("../../../logo-Kortex.png")) {
            let image = image.thumbnail(160, 160).to_rgba8();
            let size = [image.width() as usize, image.height() as usize];
            app.logo = Some(ctx.load_texture(
                "kortex_logo",
                egui::ColorImage::from_rgba_unmultiplied(size, image.as_raw()),
                Default::default(),
            ));
        }
        app
    }
    pub fn for_test() -> Self {
        Self::with_preferences(Preferences::default())
    }
    fn with_preferences(prefs: Preferences) -> Self {
        let tree = prefs
            .layout
            .clone()
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_else(default_layout);
        Self {
            prefs,
            path: None,
            tree: Some(tree),
            connections: BTreeMap::new(),
            documents: vec![],
            detached: vec![],
            active: None,
            selected: None,
            next_id: 1,
            jobs: BTreeMap::new(),
            actions: vec![],
            messages: VecDeque::new(),
            search: String::new(),
            form: None,
            settings: false,
            last_save: Instant::now(),
            close_document: None,
            close_app: false,
            allow_close: false,
            logo: None,
            navigation: vec![],
            navigation_index: 0,
        }
    }
    pub fn open_sqlite_path(&mut self, path: PathBuf) {
        let name = path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        self.actions.push(Action::Connect(
            Profile {
                id: new_id(),
                name,
                engine: Engine::Sqlite {
                    path: path.to_string_lossy().into_owned(),
                },
                remember_password: false,
            },
            String::new(),
        ));
    }
    fn id(&mut self) -> u64 {
        self.next_id += 1;
        self.next_id
    }
    fn message(&mut self, message: String) {
        self.messages.push_back(message);
        if self.messages.len() > 100 {
            self.messages.pop_front();
        }
    }
    fn save(&mut self) {
        for (id, conn) in &self.connections {
            self.prefs.diagrams.insert(id.clone(), conn.diagram.clone());
        }
        self.prefs.layout = self
            .tree
            .as_ref()
            .and_then(|t| serde_json::to_value(t).ok());
        if let Some(path) = &self.path
            && let Err(e) = storage::save(path, &self.prefs)
        {
            self.message(format!("Sauvegarde : {e}"));
        }
        self.last_save = Instant::now();
    }
    fn wake(ctx: &egui::Context) -> Arc<dyn Fn() + Send + Sync> {
        let ctx = ctx.clone();
        Arc::new(move || ctx.request_repaint())
    }
    fn connect(&mut self, profile: Profile, password: String, ctx: &egui::Context) {
        if self.connections.contains_key(&profile.id) {
            self.message(
                "Cette connexion est déjà ouverte. Reconnectez-la ou créez un autre profil.".into(),
            );
            return;
        }
        if profile.name.trim().is_empty() {
            self.message("Donnez un nom à la connexion.".into());
            return;
        }
        if profile.remember_password {
            if let Err(e) = NativeSecrets.set(&profile.id, &password) {
                self.message(format!("Mot de passe non mémorisé : {e}"));
            }
        } else if let Err(e) = NativeSecrets.delete(&profile.id) {
            self.message(format!("Gestionnaire de secrets : {e}"));
        }
        self.prefs.profiles.retain(|p| p.id != profile.id);
        self.prefs.profiles.push(profile.clone());
        let diagram = self
            .prefs
            .diagrams
            .get(&profile.id)
            .cloned()
            .unwrap_or_default();
        self.connections.insert(
            profile.id.clone(),
            Connection {
                worker: Worker::start(profile.clone(), password.clone(), Self::wake(ctx)),
                profile,
                password,
                catalog: Catalog::default(),
                ready: false,
                catalog_pending: false,
                last_schema: Instant::now(),
                diagram,
                history: LayoutHistory::default(),
                error: None,
                failures: 0,
                retry_at: None,
            },
        );
        self.save();
    }
    fn request_catalog(&mut self, connection: &str) {
        let id = self.id();
        if let Some(conn) = self.connections.get_mut(connection) {
            if conn.catalog_pending || !conn.ready {
                return;
            }
            match conn.worker.submit(Request {
                id,
                operation: Operation::Catalog,
            }) {
                Ok(()) => {
                    conn.catalog_pending = true;
                    self.jobs.insert(id, Job::Catalog(connection.into()));
                }
                Err(e) => self.message(e.to_string()),
            }
        }
    }
    fn request_page(&mut self, document: u64) {
        let id = self.id();
        let Some(doc) = self.documents.iter_mut().find(|d| d.id == document) else {
            return;
        };
        let DocumentKind::Data(view) = &mut doc.kind else {
            return;
        };
        if view.pending.is_some() || view.applying || !view.schema_valid {
            return;
        }
        let Some(conn) = self.connections.get(&doc.connection) else {
            return;
        };
        if !conn.ready {
            return;
        }
        match conn.worker.submit(Request {
            id,
            operation: Operation::Page {
                table: view.table.clone(),
                query: view.query.clone(),
            },
        }) {
            Ok(()) => {
                view.pending = Some(id);
                view.refresh_requested = false;
                self.jobs.insert(id, Job::Page(document));
            }
            Err(e) => view.error = Some(e.to_string()),
        }
    }
    fn apply_changes(&mut self, document: u64) {
        let id = self.id();
        let Some(doc) = self.documents.iter_mut().find(|d| d.id == document) else {
            return;
        };
        let DocumentKind::Data(view) = &mut doc.kind else {
            return;
        };
        if view.applying || view.pending.is_some() || !view.schema_valid {
            return;
        }
        let Some(conn) = self.connections.get(&doc.connection) else {
            return;
        };
        match conn.worker.submit(Request {
            id,
            operation: Operation::Apply {
                table: view.table.clone(),
                changes: view.grid.changes(),
            },
        }) {
            Ok(()) => {
                view.applying = true;
                self.jobs.insert(id, Job::Apply(document));
            }
            Err(e) => view.error = Some(e.to_string()),
        }
    }
    fn poll(&mut self, ctx: &egui::Context) {
        let mut incoming = vec![];
        for (id, conn) in &self.connections {
            for response in conn.worker.receiver.try_iter().take(128) {
                incoming.push((id.clone(), response));
            }
        }
        let mut refresh_connections = vec![];
        for (connection, response) in incoming {
            match response.payload {
                Payload::Connected => {
                    if let Some(conn) = self.connections.get_mut(&connection) {
                        conn.ready = true;
                        conn.error = None;
                        conn.retry_at = None;
                        conn.failures = 0;
                    }
                    self.request_catalog(&connection);
                }
                Payload::Failed(e) => {
                    if let Some(conn) = self.connections.get_mut(&connection) {
                        conn.error = Some(e.to_string());
                        conn.ready = false;
                        conn.failures += 1;
                        conn.retry_at = Some(
                            Instant::now() + Duration::from_secs(2u64.pow(conn.failures.min(5))),
                        );
                    }
                    self.message(format!("Connexion : {e}"));
                }
                payload => {
                    let Some(job) = self.jobs.remove(&response.id) else {
                        continue;
                    };
                    match (job, payload) {
                        (Job::Catalog(id), Payload::Catalog(result)) => {
                            if let Some(conn) = self.connections.get_mut(&id) {
                                conn.catalog_pending = false;
                                conn.last_schema = Instant::now();
                                match result {
                                    Ok(catalog) => {
                                        conn.diagram.reconcile(&catalog.tables);
                                        conn.catalog = catalog;
                                        conn.error = None;
                                    }
                                    Err(e) => {
                                        conn.error = Some(e.to_string());
                                        if matches!(conn.profile.engine, Engine::Postgres { .. }) {
                                            conn.ready = false;
                                            conn.failures += 1;
                                            conn.retry_at = Some(
                                                Instant::now()
                                                    + Duration::from_secs(
                                                        2u64.pow(conn.failures.min(5)),
                                                    ),
                                            );
                                        }
                                    }
                                }
                            }
                            for doc in &mut self.documents {
                                if doc.connection == id
                                    && let DocumentKind::Data(view) = &mut doc.kind
                                {
                                    let table = self.connections.get(&id).and_then(|c| {
                                        c.catalog
                                            .tables
                                            .iter()
                                            .find(|t| t.stable_id == view.table.stable_id)
                                    });
                                    view.reconcile_table(table);
                                }
                            }
                        }
                        (Job::Page(id), Payload::Page(result)) => {
                            if let Some(Document {
                                kind: DocumentKind::Data(view),
                                ..
                            }) = self.documents.iter_mut().find(|d| d.id == id)
                                && view.pending == Some(response.id)
                            {
                                view.pending = None;
                                view.last_refresh = Instant::now();
                                match result {
                                    Ok(page) => {
                                        view.grid.receive(page);
                                        if !view.grid.dirty() {
                                            view.error = None;
                                        }
                                    }
                                    Err(e) => view.error = Some(e.to_string()),
                                }
                            }
                        }
                        (Job::Apply(id), Payload::Applied(result)) => {
                            if let Some(doc) = self.documents.iter_mut().find(|d| d.id == id)
                                && let DocumentKind::Data(view) = &mut doc.kind
                            {
                                view.applying = false;
                                match result {
                                    Ok(count) => {
                                        view.grid.discard();
                                        view.selected_cell = None;
                                        view.error = None;
                                        refresh_connections.push(doc.connection.clone());
                                        self.message(format!("{count} ligne(s) enregistrée(s)."));
                                    }
                                    Err(e) => {
                                        let message = format!(
                                            "Écriture non confirmée : {e}. Aucune relance automatique."
                                        );
                                        view.error = Some(message.clone());
                                        self.message("Échec d’enregistrement. Les détails restent dans l’onglet concerné.".into());
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
        for doc in &mut self.documents {
            if let DocumentKind::Sql(view) = &mut doc.kind {
                let events: Vec<_> = view
                    .worker
                    .as_ref()
                    .map(|w| w.receiver.try_iter().take(128).collect())
                    .unwrap_or_default();
                for response in events {
                    match response.payload {
                        Payload::Connected => view.connected = true,
                        Payload::Failed(e) => {
                            view.error = Some(e.to_string());
                            view.connected = false;
                            view.running = false;
                        }
                        Payload::Sql(event) if response.id == view.request => {
                            if matches!(event, SqlEvent::Finished { .. }) {
                                refresh_connections.push(doc.connection.clone());
                            }
                            view.receive(event);
                        }
                        _ => {}
                    }
                }
            }
        }
        refresh_connections.sort();
        refresh_connections.dedup();
        for connection in refresh_connections {
            self.request_catalog(&connection);
            for doc in &mut self.documents {
                if doc.connection == connection
                    && let DocumentKind::Data(view) = &mut doc.kind
                {
                    view.refresh_requested = true;
                }
            }
        }
        let active = ctx.input(|i| i.viewport().focused.unwrap_or(true));
        let factor = if active { 1.0 } else { 10.0 };
        let catalogs: Vec<_> = self
            .connections
            .iter()
            .filter(|(_, c)| {
                c.ready
                    && !c.catalog_pending
                    && matches!(c.profile.engine, Engine::Postgres { .. })
                    && c.last_schema.elapsed().as_secs_f32()
                        > self.prefs.schema_refresh_secs.max(1.0) * factor
            })
            .map(|(id, _)| id.clone())
            .collect();
        for id in catalogs {
            self.request_catalog(&id);
        }
        let pages: Vec<_> = self
            .documents
            .iter()
            .filter_map(|doc| {
                if let DocumentKind::Data(view) = &doc.kind {
                    let pg = self
                        .connections
                        .get(&doc.connection)
                        .is_some_and(|c| matches!(c.profile.engine, Engine::Postgres { .. }));
                    if view.pending.is_none()
                        && !view.applying
                        && view.schema_valid
                        && (view.refresh_requested
                            || (pg
                                && self.active == Some(doc.id)
                                && view.last_refresh.elapsed().as_secs_f32()
                                    > self.prefs.data_refresh_secs.max(0.25) * factor))
                    {
                        return Some(doc.id);
                    }
                }
                None
            })
            .collect();
        for id in pages {
            self.request_page(id);
        }
        let reconnect: Vec<_> = self
            .connections
            .iter()
            .filter(|(_, c)| c.retry_at.is_some_and(|at| Instant::now() >= at))
            .map(|(id, _)| id.clone())
            .collect();
        for id in reconnect {
            self.reconnect(&id, ctx);
        }
    }
    fn reconnect(&mut self, id: &str, ctx: &egui::Context) {
        if let Some(conn) = self.connections.get_mut(id) {
            conn.worker =
                Worker::start(conn.profile.clone(), conn.password.clone(), Self::wake(ctx));
            conn.ready = false;
            conn.catalog_pending = false;
            conn.retry_at = None;
            let mut affected_documents = vec![];
            for doc in &mut self.documents {
                if doc.connection == id
                    && let DocumentKind::Data(view) = &mut doc.kind
                {
                    affected_documents.push(doc.id);
                    view.pending = None;
                    if view.applying {
                        view.applying = false;
                        view.error = Some("Connexion interrompue pendant une écriture : issue inconnue. Vérifiez la base avant de réessayer.".into());
                    }
                    view.refresh_requested = true;
                }
            }
            self.jobs.retain(|_, job| match job {
                Job::Catalog(connection) => connection != id,
                Job::Page(document) | Job::Apply(document) => {
                    !affected_documents.contains(document)
                }
            });
        }
    }
    fn process_actions(&mut self, ctx: &egui::Context) {
        for action in std::mem::take(&mut self.actions) {
            match action {
                Action::Connect(profile, password) => self.connect(profile, password, ctx),
                Action::OpenTable(connection, stable) => self.open_table(connection, stable),
                Action::Diagram(connection) => {
                    if let Some(doc) = self.documents.iter().find(|d| {
                        d.connection == connection && matches!(d.kind, DocumentKind::Diagram(_))
                    }) {
                        self.active = Some(doc.id);
                    } else {
                        let id = self.id();
                        self.documents.push(Document {
                            id,
                            connection,
                            title: "MCD".into(),
                            kind: DocumentKind::Diagram(DiagramView::default()),
                        });
                        self.active = Some(id);
                    }
                }
                Action::Sql(connection) => {
                    let id = self.id();
                    let mut view = SqlView::new(self.prefs.sql_limit);
                    if let Some(conn) = self.connections.get(&connection) {
                        view.worker = Some(Worker::start(
                            conn.profile.clone(),
                            conn.password.clone(),
                            Self::wake(ctx),
                        ));
                    }
                    self.documents.push(Document {
                        id,
                        connection,
                        title: format!("SQL {id}"),
                        kind: DocumentKind::Sql(view),
                    });
                    self.active = Some(id);
                }
                Action::Refresh(id) => self.request_page(id),
                Action::Apply(id) => self.apply_changes(id),
                Action::Execute(document) => {
                    let id = self.id();
                    if let Some(Document {
                        kind: DocumentKind::Sql(view),
                        ..
                    }) = self.documents.iter_mut().find(|d| d.id == document)
                    {
                        let sql = view.execution_text();
                        if sql.trim().is_empty() {
                            continue;
                        }
                        if let Some(worker) = &view.worker {
                            match worker.submit(Request {
                                id,
                                operation: Operation::Sql {
                                    sql: sql.clone(),
                                    limit: view.limit.clamp(1, 100000),
                                },
                            }) {
                                Ok(()) => {
                                    view.request = id;
                                    view.running = true;
                                    view.clear_results();
                                    view.error = None;
                                    view.history.push(sql);
                                    if view.history.len() > 50 {
                                        view.history.remove(0);
                                    }
                                }
                                Err(e) => view.error = Some(e.to_string()),
                            }
                        }
                    }
                }
                Action::Cancel(id) => {
                    if let Some(Document {
                        kind: DocumentKind::Sql(view),
                        ..
                    }) = self.documents.iter().find(|d| d.id == id)
                        && let Some(worker) = &view.worker
                    {
                        worker.cancel();
                    }
                }
                Action::Close(id) => {
                    if self.documents.iter().any(|d| {
                        d.id == id
                            && match &d.kind {
                                DocumentKind::Data(v) => v.grid.dirty() || v.applying,
                                DocumentKind::Sql(_) => true,
                                _ => false,
                            }
                    }) {
                        self.close_document = Some(id);
                    } else {
                        self.documents.retain(|d| d.id != id);
                        if self.active == Some(id) {
                            self.active = self.documents.last().map(|d| d.id);
                        }
                    }
                }
                Action::Forget(id) => {
                    if self.documents.iter().any(|d| d.connection == id) {
                        self.message(
                            "Fermez les onglets de cette connexion avant de l’oublier.".into(),
                        );
                    } else {
                        self.connections.remove(&id);
                        self.prefs.profiles.retain(|p| p.id != id);
                        if let Err(e) = NativeSecrets.delete(&id) {
                            self.message(e.to_string());
                        }
                        self.save();
                    }
                }
                Action::Reconnect(id) => self.reconnect(&id, ctx),
                Action::SwitchDatabase(id, database) => {
                    if let Some(conn) = self.connections.get(&id) {
                        let mut profile = conn.profile.clone();
                        profile.id = new_id();
                        profile.name = format!("{} / {database}", profile.name);
                        if let Engine::Postgres { database: db, .. } = &mut profile.engine {
                            *db = database;
                        }
                        let password = conn.password.clone();
                        self.connect(profile, password, ctx);
                    }
                }
            }
        }
    }
    fn open_table(&mut self, connection: String, stable: String) {
        let Some(table) = self
            .connections
            .get(&connection)
            .and_then(|c| c.catalog.tables.iter().find(|t| t.stable_id == stable))
            .cloned()
        else {
            return;
        };
        self.selected = Some((connection.clone(), stable.clone()));
        if self.navigation.last() != Some(&(connection.clone(), stable.clone())) {
            self.navigation.push((connection.clone(), stable.clone()));
            self.navigation_index = self.navigation.len() - 1;
        }
        if let Some(doc) = self.documents.iter().find(|d| {
            d.connection == connection
                && matches!(&d.kind,DocumentKind::Data(v) if v.table.stable_id==stable)
        }) {
            self.active = Some(doc.id);
            return;
        }
        let id = self.id();
        self.documents.push(Document {
            id,
            connection,
            title: table.id.name.clone(),
            kind: DocumentKind::Data(Box::new(DataView::new(table))),
        });
        self.active = Some(id);
    }
    pub fn render(&mut self, ui: &mut Ui) {
        let ctx = ui.ctx().clone();
        self.poll(&ctx);
        egui::Panel::top("top_bar").resizable(false).show(ui, |ui| {
            ui.horizontal(|ui| {
                if let Some(logo) = &self.logo {
                    ui.image((logo.id(), egui::vec2(38.0, 38.0)));
                }
                ui.vertical(|ui| {
                    ui.label(RichText::new("KORTEX").size(20.0).strong());
                    ui.label(RichText::new("SUPERVISOR").size(10.0).color(theme::RED));
                });
                ui.separator();
                if ui.button("Nouvelle connexion").clicked() {
                    self.form = Some(ConnectionForm::default());
                }
                if ui.button("Ouvrir un fichier SQLite").clicked()
                    && let Some(path) = rfd::FileDialog::new()
                        .add_filter("SQLite", &["db", "sqlite", "sqlite3"])
                        .pick_file()
                {
                    self.open_sqlite_path(path);
                }
                if ui.button("Paramètres").clicked() {
                    self.settings = true;
                }
                if ui.button("Disposition initiale").clicked() {
                    self.tree = Some(default_layout());
                }
            });
        });
        egui::Panel::bottom("status")
            .resizable(false)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.colored_label(theme::YELLOW, "●");
                    ui.label(format!(
                        "{} connexion(s) · {} onglet(s)",
                        self.connections.len(),
                        self.documents.len()
                    ));
                    if let Some(doc) = self.documents.iter().find(|d| Some(d.id) == self.active)
                        && let Some(conn) = self.connections.get(&doc.connection)
                    {
                        ui.separator();
                        ui.strong(&conn.profile.name);
                        ui.label(&conn.catalog.server_version);
                    }
                });
            });
        egui::CentralPanel::default().show(ui, |ui| {
            let mut tree = self.tree.take().unwrap_or_else(default_layout);
            tree.ui(self, ui);
            self.tree = Some(tree);
        });
        self.connection_dialog(&ctx);
        self.settings_dialog(&ctx);
        self.close_dialogs(&ctx);
        self.show_detached(&ctx);
        self.process_actions(&ctx);
        if self.last_save.elapsed() > Duration::from_secs(5) && !ctx.input(|i| i.pointer.any_down())
        {
            self.save();
        }
        ctx.request_repaint_after(Duration::from_millis(250));
    }
    fn explorer(&mut self, ui: &mut Ui) {
        ui.add(
            egui::TextEdit::singleline(&mut self.search)
                .hint_text("Rechercher une table…")
                .desired_width(f32::INFINITY),
        );
        egui::ScrollArea::vertical().show(ui, |ui| {
            for profile in self.prefs.profiles.clone() {
                ui.push_id(&profile.id, |ui| {
                    let connected = self.connections.get(&profile.id);
                    let title = format!(
                        "{} {}",
                        if connected.is_some_and(|c| c.ready) {
                            "●"
                        } else {
                            "○"
                        },
                        profile.name
                    );
                    egui::CollapsingHeader::new(title)
                        .default_open(true)
                        .show(ui, |ui| {
                            ui.horizontal_wrapped(|ui| {
                                if let Some(conn) = connected {
                                    if !conn.ready {
                                        ui.spinner();
                                    }
                                    if ui.small_button("MCD").clicked() {
                                        self.actions.push(Action::Diagram(profile.id.clone()));
                                    }
                                    if ui.small_button("SQL").clicked() {
                                        self.actions.push(Action::Sql(profile.id.clone()));
                                    }
                                    if ui.small_button("↻").on_hover_text("Reconnecter").clicked()
                                    {
                                        self.actions.push(Action::Reconnect(profile.id.clone()));
                                    }
                                } else if ui.button("Connecter").clicked() {
                                    match &profile.engine {
                                        Engine::Sqlite { .. } => self
                                            .actions
                                            .push(Action::Connect(profile.clone(), String::new())),
                                        _ => {
                                            let password =
                                                NativeSecrets.get(&profile.id).ok().flatten();
                                            if let Some(password) = password {
                                                self.actions.push(Action::Connect(
                                                    profile.clone(),
                                                    password,
                                                ));
                                            } else {
                                                self.form =
                                                    Some(ConnectionForm::from_profile(&profile));
                                            }
                                        }
                                    }
                                }
                                if connected.is_none() && ui.small_button("Modifier").clicked() {
                                    self.form = Some(ConnectionForm::from_profile(&profile));
                                }
                                if ui.small_button("Oublier").clicked() {
                                    self.actions.push(Action::Forget(profile.id.clone()));
                                }
                            });
                            if let Some(conn) = connected {
                                if let Some(error) = &conn.error {
                                    ui.colored_label(theme::RED, error);
                                }
                                if matches!(profile.engine, Engine::Postgres { .. }) {
                                    ui.menu_button("Bases disponibles", |ui| {
                                        for db in &conn.catalog.databases {
                                            if ui.button(db).clicked() {
                                                self.actions.push(Action::SwitchDatabase(
                                                    profile.id.clone(),
                                                    db.clone(),
                                                ));
                                                ui.close();
                                            }
                                        }
                                    });
                                }
                                let schemas: std::collections::BTreeSet<_> =
                                    conn.catalog.tables.iter().map(|t| &t.id.schema).collect();
                                for schema in schemas {
                                    egui::CollapsingHeader::new(schema).default_open(true).show(
                                        ui,
                                        |ui| {
                                            for table in conn.catalog.tables.iter().filter(|t| {
                                                &t.id.schema == schema
                                                    && t.id
                                                        .label()
                                                        .to_lowercase()
                                                        .contains(&self.search.to_lowercase())
                                            }) {
                                                let selected = self.selected.as_ref()
                                                    == Some(&(
                                                        profile.id.clone(),
                                                        table.stable_id.clone(),
                                                    ));
                                                let label = format!(
                                                    "{} {}",
                                                    if table.is_view { "◈" } else { "▦" },
                                                    table.id.name
                                                );
                                                if ui
                                                    .selectable_label(selected, label)
                                                    .on_hover_text(format!(
                                                        "{} colonnes · {} index",
                                                        table.columns.len(),
                                                        table.indexes.len()
                                                    ))
                                                    .clicked()
                                                {
                                                    self.actions.push(Action::OpenTable(
                                                        profile.id.clone(),
                                                        table.stable_id.clone(),
                                                    ));
                                                }
                                            }
                                        },
                                    );
                                }
                            }
                        });
                });
            }
            if self.prefs.profiles.is_empty() {
                ui.add_space(20.0);
                ui.label(
                    RichText::new("Ajoutez une connexion PostgreSQL ou ouvrez un fichier SQLite.")
                        .color(theme::MUTED),
                );
            }
        });
    }
    fn workspace(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            if ui
                .add_enabled(self.navigation_index > 0, egui::Button::new("←"))
                .clicked()
            {
                self.navigation_index -= 1;
                let (connection, stable) = self.navigation[self.navigation_index].clone();
                self.actions.push(Action::OpenTable(connection, stable));
            }
            egui::ScrollArea::horizontal().show(ui, |ui| {
                ui.horizontal(|ui| {
                    for doc in &self.documents {
                        let dirty = matches!(&doc.kind,DocumentKind::Data(v) if v.grid.dirty());
                        let connection = self
                            .connections
                            .get(&doc.connection)
                            .map(|c| c.profile.name.as_str())
                            .unwrap_or("?");
                        if ui
                            .selectable_label(
                                self.active == Some(doc.id),
                                format!("{}{}", doc.title, if dirty { " •" } else { "" }),
                            )
                            .on_hover_text(connection)
                            .clicked()
                        {
                            self.active = Some(doc.id);
                        }
                        if ui
                            .small_button("×")
                            .on_hover_text("Fermer l’onglet")
                            .clicked()
                        {
                            self.actions.push(Action::Close(doc.id));
                        }
                    }
                });
            });
        });
        ui.separator();
        let Some(index) = self
            .documents
            .iter()
            .position(|d| Some(d.id) == self.active)
        else {
            ui.vertical_centered(|ui| {
                ui.add_space(75.0);
                ui.label(
                    RichText::new("Votre base, dans son ensemble.")
                        .size(27.0)
                        .strong(),
                );
                ui.add_space(12.0);
                ui.label("Explorez les données, suivez les relations et exécutez vos requêtes.");
                ui.add_space(25.0);
                ui.horizontal_wrapped(|ui| {
                    ui.label(theme::heading("01  CONNECTER"));
                    ui.label("PostgreSQL ou SQLite");
                    ui.label(theme::heading("02  EXPLORER"));
                    ui.label("Données et structure");
                    ui.label(theme::heading("03  PILOTER"));
                    ui.label("Tableur et SQL");
                });
            });
            return;
        };
        let mut doc = self.documents.remove(index);
        if let Some(conn) = self.connections.get(&doc.connection) {
            ui.label(
                RichText::new(format!("{}  /  {}", conn.profile.name, doc.title))
                    .small()
                    .color(theme::MUTED),
            );
        }
        ui.push_id(doc.id, |ui| match &mut doc.kind {
            DocumentKind::Data(view) => {
                let action = grid::show(ui, view);
                if action.refresh {
                    self.actions.push(Action::Refresh(doc.id));
                }
                if action.apply {
                    self.actions.push(Action::Apply(doc.id));
                }
            }
            DocumentKind::Sql(view) => {
                let empty = Catalog::default();
                let catalog = self
                    .connections
                    .get(&doc.connection)
                    .map(|c| &c.catalog)
                    .unwrap_or(&empty);
                let action = sql::show(ui, view, catalog);
                if action.execute {
                    self.actions.push(Action::Execute(doc.id));
                }
                if action.cancel {
                    self.actions.push(Action::Cancel(doc.id));
                }
            }
            DocumentKind::Diagram(view) => {
                if let Some(conn) = self.connections.get_mut(&doc.connection) {
                    let action = mcd::show(
                        ui,
                        &conn.catalog,
                        &mut conn.diagram,
                        &mut conn.history,
                        view,
                        None,
                    );
                    self.diagram_action(doc.connection.clone(), action);
                }
            }
        });
        self.documents.insert(index, doc);
    }
    fn diagram_action(&mut self, connection: String, action: mcd::DiagramAction) {
        if let Some(stable) = action.inspect {
            self.selected = Some((connection.clone(), stable));
        }
        if let Some(stable) = action.open_table {
            self.actions
                .push(Action::OpenTable(connection.clone(), stable));
        }
        if let Some((title, members)) = action.detach {
            let id = self.id();
            self.detached.push(Detached {
                id,
                connection,
                title,
                members,
                view: DiagramView::default(),
            });
        }
    }
    fn inspector(&mut self, ui: &mut Ui) {
        let selected = self.selected.as_ref().and_then(|(conn, table)| {
            self.connections
                .get(conn)
                .and_then(|c| c.catalog.tables.iter().find(|t| t.stable_id == *table))
        });
        egui::ScrollArea::both().show(ui, |ui| {
            if let Some(table) = selected {
                ui.label(theme::heading("OBJET SÉLECTIONNÉ"));
                ui.heading(&table.id.name);
                ui.label(&table.id.schema);
                if !table.comment.is_empty() {
                    ui.label(&table.comment);
                }
                if let Some(count) = table.estimated_rows {
                    ui.label(format!("≈ {} lignes (estimation)", count.max(0)));
                }
                ui.separator();
                ui.label(theme::heading("COLONNES"));
                for col in &table.columns {
                    egui::CollapsingHeader::new(format!(
                        "{}{}",
                        theme::badges(table, col),
                        col.name
                    ))
                    .show(ui, |ui| {
                        ui.monospace(&col.data_type);
                        ui.label(if col.nullable {
                            "NULL autorisé"
                        } else {
                            "NOT NULL"
                        });
                        if let Some(default) = &col.default {
                            ui.monospace(format!("DEFAULT {default}"));
                        }
                        if !col.comment.is_empty() {
                            ui.label(&col.comment);
                        }
                    });
                }
                ui.separator();
                ui.label(theme::heading("INDEX"));
                for index in &table.indexes {
                    egui::CollapsingHeader::new(format!(
                        "{} {}",
                        if index.unique { "◇" } else { "≡" },
                        index.name
                    ))
                    .show(ui, |ui| {
                        ui.monospace(&index.definition);
                    });
                }
                ui.separator();
                ui.label(theme::heading("CONTRAINTES"));
                for constraint in &table.constraints {
                    egui::CollapsingHeader::new(&constraint.name).show(ui, |ui| {
                        ui.monospace(&constraint.definition);
                    });
                }
                if !table.definition.is_empty() {
                    ui.separator();
                    ui.label(theme::heading("DÉFINITION SQL"));
                    ui.monospace(&table.definition);
                }
            } else {
                ui.label(
                    "Sélectionnez une table pour inspecter ses colonnes, index et contraintes.",
                );
            }
        });
    }
    fn connection_dialog(&mut self, ctx: &egui::Context) {
        let Some(mut form) = self.form.take() else {
            return;
        };
        let mut open = true;
        let mut connect = false;
        egui::Window::new("Connexion à une base")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_width(430.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut form.sqlite, false, "PostgreSQL");
                    ui.selectable_value(&mut form.sqlite, true, "SQLite");
                });
                egui::Grid::new("connection_form")
                    .num_columns(2)
                    .spacing([15.0, 10.0])
                    .show(ui, |ui| {
                        ui.label("Nom");
                        ui.text_edit_singleline(&mut form.name);
                        ui.end_row();
                        if form.sqlite {
                            ui.label("Fichier");
                            ui.horizontal(|ui| {
                                ui.text_edit_singleline(&mut form.path);
                                if ui.button("…").clicked()
                                    && let Some(p) = rfd::FileDialog::new()
                                        .add_filter("SQLite", &["db", "sqlite", "sqlite3"])
                                        .pick_file()
                                {
                                    form.path = p.to_string_lossy().into_owned();
                                }
                            });
                            ui.end_row();
                        } else {
                            ui.label("Hôte");
                            ui.text_edit_singleline(&mut form.host);
                            ui.end_row();
                            ui.label("Port");
                            ui.add(egui::DragValue::new(&mut form.port).range(1..=65535));
                            ui.end_row();
                            ui.label("Base");
                            ui.text_edit_singleline(&mut form.database);
                            ui.end_row();
                            ui.label("Utilisateur");
                            ui.text_edit_singleline(&mut form.user);
                            ui.end_row();
                            ui.label("Mot de passe");
                            ui.add(egui::TextEdit::singleline(&mut form.password).password(true));
                            ui.end_row();
                            ui.label("Stockage sécurisé");
                            ui.checkbox(&mut form.remember, "Mémoriser le mot de passe");
                            ui.end_row();
                            ui.label("Chiffrement");
                            ui.checkbox(&mut form.tls, "TLS avec validation du serveur");
                            ui.end_row();
                            ui.label("CA personnalisée");
                            ui.horizontal(|ui| {
                                ui.text_edit_singleline(&mut form.ca);
                                if ui.button("…").clicked()
                                    && let Some(p) = rfd::FileDialog::new()
                                        .add_filter("Certificat PEM", &["pem", "crt"])
                                        .pick_file()
                                {
                                    form.ca = p.to_string_lossy().into_owned();
                                }
                            });
                            ui.end_row();
                        }
                    });
                ui.separator();
                if ui.button("Connecter").clicked() {
                    connect = true;
                }
            });
        if connect {
            self.actions
                .push(Action::Connect(form.profile(), form.password));
        } else if open {
            self.form = Some(form);
        }
    }
    fn settings_dialog(&mut self, ctx: &egui::Context) {
        egui::Window::new("Paramètres")
            .open(&mut self.settings)
            .show(ctx, |ui| {
                ui.label("Actualisation PostgreSQL");
                ui.horizontal(|ui| {
                    ui.label("Données (secondes)");
                    ui.add(
                        egui::DragValue::new(&mut self.prefs.data_refresh_secs)
                            .range(0.25..=120.0)
                            .speed(0.25),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Structure (secondes)");
                    ui.add(
                        egui::DragValue::new(&mut self.prefs.schema_refresh_secs)
                            .range(1.0..=300.0),
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Lignes SQL affichées par défaut");
                    ui.add(egui::DragValue::new(&mut self.prefs.sql_limit).range(1..=100000));
                });
                ui.separator();
                ui.label("Les fichiers SQLite ne sont pas surveillés automatiquement.");
                ui.label("Les résultats SQL et les brouillons restent uniquement en mémoire.");
            });
    }
    fn close_dialogs(&mut self, ctx: &egui::Context) {
        if let Some(id) = self.close_document {
            egui::Window::new("Fermer cet onglet ?").collapsible(false).show(ctx, |ui| {
                ui.label("Les modifications en attente seront perdues. Fermer une session SQL annule ses transactions non validées.");
                ui.horizontal(|ui| {
                    if ui.button("Conserver l’onglet").clicked() { self.close_document = None; }
                    if ui.button("Fermer et abandonner").clicked() {
                        self.documents.retain(|d| d.id != id);
                        self.active = self.documents.last().map(|d| d.id);
                        self.close_document = None;
                    }
                });
            });
        }
        if self.close_app {
            egui::Window::new("Quitter Kortex Supervisor ?").collapsible(false).show(ctx, |ui| {
                ui.label("Des brouillons ou sessions SQL sont ouverts. Les transactions non validées seront annulées.");
                ui.horizontal(|ui| {
                    if ui.button("Rester").clicked() { self.close_app = false; }
                    if ui.button("Quitter et abandonner").clicked() {
                        self.allow_close = true;
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
            });
        }
    }
    fn show_detached(&mut self, ctx: &egui::Context) {
        let mut windows = std::mem::take(&mut self.detached);
        for mut window in windows.drain(..) {
            let mut open = true;
            let mut action = None;
            if let Some(conn) = self.connections.get_mut(&window.connection) {
                ctx.show_viewport_immediate(
                    egui::ViewportId::from_hash_of(("mcd", window.id)),
                    egui::ViewportBuilder::default()
                        .with_title(format!("{} — Kortex MCD", window.title))
                        .with_inner_size([1100.0, 740.0]),
                    |ui, _| {
                        if ui.ctx().input(|i| i.viewport().close_requested()) {
                            open = false;
                        }
                        egui::CentralPanel::default().show(ui, |ui| {
                            action = Some(mcd::show(
                                ui,
                                &conn.catalog,
                                &mut conn.diagram,
                                &mut conn.history,
                                &mut window.view,
                                Some(&window.members),
                            ));
                        });
                    },
                );
            }
            if let Some(action) = action {
                self.diagram_action(window.connection.clone(), action);
            }
            if open {
                self.detached.push(window);
            }
        }
    }
}
impl egui_tiles::Behavior<Pane> for KortexApp {
    fn simplification_options(&self) -> egui_tiles::SimplificationOptions {
        egui_tiles::SimplificationOptions {
            all_panes_must_have_tabs: true,
            ..Default::default()
        }
    }
    fn tab_title_for_pane(&mut self, pane: &Pane) -> egui::WidgetText {
        match pane {
            Pane::Explorer => "EXPLORATEUR",
            Pane::Workspace => "ESPACE DE TRAVAIL",
            Pane::Inspector => "INSPECTEUR",
            Pane::Messages => "ACTIVITÉ",
        }
        .into()
    }
    fn pane_ui(
        &mut self,
        ui: &mut Ui,
        _tile: egui_tiles::TileId,
        pane: &mut Pane,
    ) -> egui_tiles::UiResponse {
        egui::Frame::new().inner_margin(8).show(ui,|ui|{match pane{Pane::Explorer=>self.explorer(ui),Pane::Workspace=>self.workspace(ui),Pane::Inspector=>self.inspector(ui),Pane::Messages=>{egui::ScrollArea::vertical().stick_to_bottom(true).show(ui,|ui|{if self.messages.is_empty(){ui.label(RichText::new("Prêt. Les opérations et erreurs de connexion apparaîtront ici.").color(theme::MUTED));}for message in &self.messages{ui.label(message);}});}}});
        egui_tiles::UiResponse::None
    }
}
impl eframe::App for KortexApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll(ctx);
        if ctx.input(|i| i.viewport().close_requested())
            && !self.allow_close
            && self.documents.iter().any(|d| match &d.kind {
                DocumentKind::Data(v) => v.grid.dirty() || v.applying,
                DocumentKind::Sql(_) => true,
                _ => false,
            })
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.close_app = true;
        }
    }
    fn ui(&mut self, ui: &mut Ui, _frame: &mut eframe::Frame) {
        self.render(ui);
    }
    fn on_exit(&mut self) {
        self.save();
    }
}
fn new_id() -> String {
    format!(
        "{:x}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    )
}
fn default_layout() -> egui_tiles::Tree<Pane> {
    use egui_tiles::{Container, Linear, LinearDir, Tile, Tiles};
    let mut tiles = Tiles::default();
    let explorer = tiles.insert_pane(Pane::Explorer);
    let workspace = tiles.insert_pane(Pane::Workspace);
    let inspector = tiles.insert_pane(Pane::Inspector);
    let messages = tiles.insert_pane(Pane::Messages);
    let middle = tiles.insert_new(Tile::Container(Container::Linear(Linear::new_binary(
        LinearDir::Vertical,
        [workspace, messages],
        0.85,
    ))));
    let mut row = Linear::new(LinearDir::Horizontal, vec![explorer, middle, inspector]);
    row.shares.set_share(explorer, 0.17);
    row.shares.set_share(middle, 0.65);
    row.shares.set_share(inspector, 0.18);
    let root = tiles.insert_new(Tile::Container(Container::Linear(row)));
    egui_tiles::Tree::new(egui::Id::unique("workspace_layout"), root, tiles)
}
