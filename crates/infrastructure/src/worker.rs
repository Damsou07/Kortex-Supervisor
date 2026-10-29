use crate::{postgres::PostgresBackend, sqlite::SqliteBackend};
use kortex_application::{CancelQuery, DatabaseBackend};
use kortex_domain::*;
use std::sync::{
    Arc, Mutex,
    mpsc::{self, Receiver, SyncSender},
};

pub enum Operation {
    Catalog,
    Page { table: Table, query: PageQuery },
    Apply { table: Table, changes: ChangeSet },
    Sql { sql: String, limit: usize },
}
pub struct Request {
    pub id: u64,
    pub operation: Operation,
}
pub enum Payload {
    Connected,
    Catalog(Result<Catalog>),
    Page(Result<DataPage>),
    Applied(Result<u64>),
    Sql(SqlEvent),
    Failed(Error),
}
pub struct Response {
    pub id: u64,
    pub payload: Payload,
}
pub struct Worker {
    sender: Option<SyncSender<Request>>,
    pub receiver: Receiver<Response>,
    cancel: Arc<Mutex<Option<Arc<dyn CancelQuery>>>>,
}
impl Worker {
    pub fn start(profile: Profile, password: String, wake: Arc<dyn Fn() + Send + Sync>) -> Self {
        let (sender, commands) = mpsc::sync_channel::<Request>(32);
        let (events, receiver) = mpsc::sync_channel(64);
        let cancel = Arc::new(Mutex::new(None));
        let handle = cancel.clone();
        std::thread::spawn(move || {
            let send = |id, payload| {
                let result = events.send(Response { id, payload });
                wake();
                result.is_ok()
            };
            let backend: Result<Box<dyn DatabaseBackend>> = match &profile.engine {
                Engine::Sqlite { path } => {
                    SqliteBackend::open(path).map(|b| Box::new(b) as Box<dyn DatabaseBackend>)
                }
                engine => PostgresBackend::connect(engine, &password)
                    .map(|b| Box::new(b) as Box<dyn DatabaseBackend>),
            };
            drop(password);
            let mut backend = match backend {
                Ok(b) => b,
                Err(e) => {
                    send(0, Payload::Failed(e));
                    return;
                }
            };
            if let Ok(mut handle) = handle.lock() {
                *handle = Some(backend.cancel_handle());
            }
            if !send(0, Payload::Connected) {
                return;
            }
            while let Ok(request) = commands.recv() {
                let id = request.id;
                let payload = match request.operation {
                    Operation::Catalog => Payload::Catalog(backend.catalog()),
                    Operation::Page { table, query } => Payload::Page(backend.page(&table, &query)),
                    Operation::Apply { table, changes } => {
                        Payload::Applied(backend.apply(&table, &changes))
                    }
                    Operation::Sql { sql, limit } => {
                        backend.execute_sql(&sql, limit, &mut |event| {
                            send(id, Payload::Sql(event));
                        });
                        continue;
                    }
                };
                if !send(id, payload) {
                    break;
                }
            }
        });
        Self {
            sender: Some(sender),
            receiver,
            cancel,
        }
    }
    pub fn submit(&self, request: Request) -> Result<()> {
        self.sender
            .as_ref()
            .ok_or_else(|| Error::Database("Connexion fermée".into()))?
            .try_send(request)
            .map_err(|_| Error::Database("Connexion indisponible ou file de travail pleine".into()))
    }
    pub fn cancel(&self) {
        if let Ok(handle) = self.cancel.lock()
            && let Some(handle) = &*handle
        {
            handle.cancel();
        }
    }
}
impl Drop for Worker {
    fn drop(&mut self) {
        self.cancel();
        self.sender.take();
    }
}
