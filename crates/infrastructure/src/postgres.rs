use futures_util::{StreamExt, pin_mut};
use kortex_application::{CancelQuery, DatabaseBackend};
use kortex_domain::*;
use rustls::pki_types::{CertificateDer, pem::PemObject};
use std::{
    collections::BTreeMap,
    sync::{Arc, OnceLock},
    time::{Duration, Instant},
};
use tokio::runtime::Runtime;
use tokio_postgres::{Client, Config, NoTls, SimpleQueryMessage, types::ToSql};
use tokio_postgres_rustls::MakeRustlsConnect;

pub struct PostgresBackend {
    client: Client,
    runtime: Arc<Runtime>,
    tls: Option<MakeRustlsConnect>,
    driver: tokio::task::JoinHandle<()>,
}
impl Drop for PostgresBackend {
    fn drop(&mut self) {
        self.driver.abort();
    }
}
struct PgCancel {
    token: tokio_postgres::CancelToken,
    runtime: Arc<Runtime>,
    tls: Option<MakeRustlsConnect>,
}
impl CancelQuery for PgCancel {
    fn cancel(&self) {
        let token = self.token.clone();
        let tls = self.tls.clone();
        self.runtime.spawn(async move {
            if let Some(tls) = tls {
                let _ = token.cancel_query(tls).await;
            } else {
                let _ = token.cancel_query(NoTls).await;
            }
        });
    }
}
fn db_error(e: tokio_postgres::Error) -> Error {
    if e.code() == Some(&tokio_postgres::error::SqlState::QUERY_CANCELED) {
        Error::Cancelled
    } else if let Some(db) = e.as_db_error() {
        Error::Database(format!("{} · {}", db.code().code(), db.message()))
    } else {
        Error::Database(e.to_string())
    }
}
fn runtime() -> Result<Arc<Runtime>> {
    static RUNTIME: OnceLock<Arc<Runtime>> = OnceLock::new();
    if let Some(runtime) = RUNTIME.get() {
        return Ok(runtime.clone());
    }
    let runtime = Arc::new(
        tokio::runtime::Builder::new_multi_thread()
            .worker_threads(4)
            .enable_all()
            .build()
            .map_err(|e| Error::Database(e.to_string()))?,
    );
    let _ = RUNTIME.set(runtime.clone());
    Ok(RUNTIME.get().cloned().unwrap_or(runtime))
}
impl PostgresBackend {
    pub fn connect(engine: &Engine, password: &str) -> Result<Self> {
        let Engine::Postgres {
            host,
            port,
            database,
            user,
            tls,
            ca_file,
        } = engine
        else {
            return Err(Error::Invalid("Profil PostgreSQL attendu".into()));
        };
        if !tls && !matches!(host.as_str(), "localhost" | "127.0.0.1" | "::1") {
            return Err(Error::Invalid("TLS est requis pour un hôte distant. Utilisez un certificat valide ou une autorité personnalisée.".into()));
        }
        let runtime = runtime()?;
        let mut config = Config::new();
        config
            .host(host)
            .port(*port)
            .dbname(database)
            .user(user)
            .password(password)
            .application_name("Kortex Supervisor")
            .connect_timeout(Duration::from_secs(8));
        let connector = if *tls {
            let mut roots = rustls::RootCertStore::empty();
            for cert in rustls_native_certs::load_native_certs().certs {
                let _ = roots.add(cert);
            }
            if !ca_file.is_empty() {
                let certificates = CertificateDer::pem_file_iter(ca_file)
                    .map_err(|e| Error::Invalid(format!("Certificat CA : {e}")))?;
                let mut count = 0;
                for cert in certificates {
                    roots
                        .add(cert.map_err(|e| Error::Invalid(e.to_string()))?)
                        .map_err(|e| Error::Invalid(e.to_string()))?;
                    count += 1;
                }
                if count == 0 {
                    return Err(Error::Invalid(
                        "Aucun certificat trouvé dans le fichier CA".into(),
                    ));
                }
            }
            let _ = rustls::crypto::ring::default_provider().install_default();
            let client_config = rustls::ClientConfig::builder()
                .with_root_certificates(roots)
                .with_no_client_auth();
            config.ssl_mode(tokio_postgres::config::SslMode::Require);
            Some(MakeRustlsConnect::new(client_config))
        } else {
            config.ssl_mode(tokio_postgres::config::SslMode::Disable);
            None
        };
        let (client, driver) = runtime.block_on(async {
            tokio::time::timeout(Duration::from_secs(10), async {
                if let Some(connector) = connector.clone() {
                    let (client, connection) = config.connect(connector).await.map_err(db_error)?;
                    Ok((
                        client,
                        tokio::spawn(async move {
                            let _ = connection.await;
                        }),
                    ))
                } else {
                    let (client, connection) = config.connect(NoTls).await.map_err(db_error)?;
                    Ok((
                        client,
                        tokio::spawn(async move {
                            let _ = connection.await;
                        }),
                    ))
                }
            })
            .await
            .map_err(|_| Error::Database("Délai de connexion dépassé".into()))?
        })?;
        Ok(Self {
            client,
            runtime,
            tls: connector,
            driver,
        })
    }
}
impl DatabaseBackend for PostgresBackend {
    fn catalog(&mut self) -> Result<Catalog> {
        self.runtime.block_on(async {
            let mut catalog = Catalog { server_version: self.client.query_one("SHOW server_version", &[]).await.map_err(db_error)?.get(0), ..Catalog::default() };
            catalog.databases = self.client.query("SELECT datname FROM pg_database WHERE datallowconn AND has_database_privilege(oid, 'CONNECT') ORDER BY datname", &[]).await.map_err(db_error)?.iter().map(|r| r.get(0)).collect();
            let rows = self.client.query("SELECT c.oid::text,n.nspname,c.relname,c.relkind::text,coalesce(obj_description(c.oid),'') AS comment,c.reltuples::bigint,CASE WHEN c.relkind IN ('v','m') THEN pg_get_viewdef(c.oid,true) ELSE '' END FROM pg_class c JOIN pg_namespace n ON n.oid=c.relnamespace WHERE c.relkind IN ('r','p','v','m','f') AND n.nspname NOT IN ('pg_catalog','information_schema') AND n.nspname NOT LIKE 'pg_toast%' ORDER BY n.nspname,c.relname", &[]).await.map_err(db_error)?;
            let mut tables = BTreeMap::<String, Table>::new();
            for row in rows {
                let stable_id: String = row.get(0); let kind: String = row.get(3);
                tables.insert(stable_id.clone(), Table { stable_id, id: ObjectId::new(row.get(1), row.get(2)), is_view: kind != "r" && kind != "p", comment: row.get(4), estimated_rows: Some(row.get(5)), definition: row.get(6), ..Table::default() });
            }
            let rows = self.client.query("SELECT a.attrelid::text,a.attname,format_type(a.atttypid,a.atttypmod),NOT a.attnotnull,pg_get_expr(d.adbin,d.adrelid),a.attgenerated <> '' OR a.attidentity = 'a',coalesce(col_description(a.attrelid,a.attnum),'') FROM pg_attribute a JOIN pg_class c ON c.oid=a.attrelid JOIN pg_namespace n ON n.oid=c.relnamespace LEFT JOIN pg_attrdef d ON d.adrelid=a.attrelid AND d.adnum=a.attnum WHERE a.attnum>0 AND NOT a.attisdropped AND c.relkind IN ('r','p','v','m','f') AND n.nspname NOT IN ('pg_catalog','information_schema') ORDER BY a.attrelid,a.attnum", &[]).await.map_err(db_error)?;
            for row in rows { if let Some(table) = tables.get_mut(row.get::<_, &str>(0)) {
                table.columns.push(Column { name: row.get(1), data_type: row.get(2), nullable: row.get(3), default: row.get(4), generated: row.get(5), comment: row.get(6) });
            } }
            let rows = self.client.query("SELECT con.oid::text,con.conrelid::text,con.conname,con.contype::text,pg_get_constraintdef(con.oid,true),ARRAY(SELECT a.attname::text FROM unnest(con.conkey) WITH ORDINALITY k(num,ord) JOIN pg_attribute a ON a.attrelid=con.conrelid AND a.attnum=k.num ORDER BY k.ord),rn.nspname,rc.relname,ARRAY(SELECT a.attname::text FROM unnest(con.confkey) WITH ORDINALITY k(num,ord) JOIN pg_attribute a ON a.attrelid=con.confrelid AND a.attnum=k.num ORDER BY k.ord),con.confmatchtype::text,con.confupdtype::text,con.confdeltype::text FROM pg_constraint con LEFT JOIN pg_class rc ON rc.oid=con.confrelid LEFT JOIN pg_namespace rn ON rn.oid=rc.relnamespace WHERE con.conrelid<>0", &[]).await.map_err(db_error)?;
            for row in rows { if let Some(table) = tables.get_mut(row.get::<_, &str>(1)) {
                let kind: String = row.get(3); let cols: Vec<String> = row.get(5);
                table.constraints.push(Constraint { name: row.get(2), kind: kind.clone(), definition: row.get(4) });
                if kind == "p" { table.primary_key = cols.clone(); }
                if kind == "u" { table.unique_keys.push(cols.clone()); }
                if kind == "f" { table.foreign_keys.push(ForeignKey { id: row.get(0), name: row.get(2), source: table.id.clone(), columns: cols, target: ObjectId::new(row.get(6), row.get(7)), target_columns: row.get(8), match_full: row.get::<_, &str>(9) == "f", on_update: referential_action(row.get(10)), on_delete: referential_action(row.get(11)) }); }
            } }
            let rows = self.client.query("SELECT i.indrelid::text,c.relname,i.indisunique,i.indpred IS NOT NULL,pg_get_indexdef(i.indexrelid),ARRAY(SELECT a.attname::text FROM unnest(i.indkey) WITH ORDINALITY k(num,ord) JOIN pg_attribute a ON a.attrelid=i.indrelid AND a.attnum=k.num WHERE k.ord<=i.indnkeyatts ORDER BY k.ord),i.indexprs IS NULL AND i.indisvalid FROM pg_index i JOIN pg_class c ON c.oid=i.indexrelid", &[]).await.map_err(db_error)?;
            for row in rows { if let Some(table) = tables.get_mut(row.get::<_, &str>(0)) {
                let index = Index { name: row.get(1), unique: row.get(2), partial: row.get(3), definition: row.get(4), columns: row.get(5) };
                if index.unique && !index.partial && row.get::<_, bool>(6) { table.unique_keys.push(index.columns.clone()); }
                table.indexes.push(index);
            } }
            catalog.tables = tables.into_values().collect(); catalog.tables.sort_by(|a,b| a.id.cmp(&b.id));
            Ok(catalog)
        })
    }
    fn page(&mut self, table: &Table, query: &PageQuery) -> Result<DataPage> {
        let built = build_page_query(table, query, Dialect::Postgres)?;
        let text_params: Vec<_> = built.params.iter().map(Value::text).collect();
        let params: Vec<&(dyn ToSql + Sync)> = text_params
            .iter()
            .map(|s| s as &(dyn ToSql + Sync))
            .collect();
        self.runtime.block_on(async {
            let results = self
                .client
                .query(&built.sql, &params)
                .await
                .map_err(db_error)?;
            let has_more = results.len() > query.limit.clamp(1, 2000);
            let rows = results
                .into_iter()
                .take(query.limit.clamp(1, 2000))
                .map(|r| {
                    let values = (0..table.columns.len())
                        .map(|i| {
                            r.get::<_, Option<String>>(i)
                                .map(Value::Text)
                                .unwrap_or(Value::Null)
                        })
                        .collect();
                    DataRow {
                        values,
                        version: if table.is_view {
                            None
                        } else {
                            r.get(table.columns.len())
                        },
                    }
                })
                .collect();
            Ok(DataPage { rows, has_more })
        })
    }
    fn apply(&mut self, table: &Table, changes: &ChangeSet) -> Result<u64> {
        if table.is_view {
            return Err(Error::Invalid("Modifiez les vues depuis SQL".into()));
        }
        self.runtime.block_on(async {
            let tx = self.client.transaction().await.map_err(db_error)?;
            let mut affected = 0;
            for change in &changes.changes {
                let mut params: Vec<Option<String>> = vec![];
                let sql = match change {
                    RowChange::Insert { values } => {
                        let mut columns = vec![];
                        let mut bindings = vec![];
                        for (name, input) in values {
                            let c = column(table, name)?;
                            if c.generated || matches!(input, InputValue::Default) {
                                continue;
                            }
                            columns.push(quote(name));
                            bindings.push(bind_input(input, c, &mut params));
                        }
                        if columns.is_empty() {
                            format!("INSERT INTO {} DEFAULT VALUES", qualified(&table.id))
                        } else {
                            format!(
                                "INSERT INTO {} ({}) VALUES ({})",
                                qualified(&table.id),
                                columns.join(","),
                                bindings.join(",")
                            )
                        }
                    }
                    RowChange::Update { original, values } => {
                        let mut sets = vec![];
                        for (name, input) in values {
                            let c = column(table, name)?;
                            if c.generated {
                                return Err(Error::Invalid(
                                    "Colonne générée non modifiable".into(),
                                ));
                            }
                            sets.push(format!(
                                "{} = {}",
                                quote(name),
                                bind_input(input, c, &mut params)
                            ));
                        }
                        if sets.is_empty() {
                            continue;
                        }
                        let predicate = original_predicate(table, original, &mut params)?;
                        format!(
                            "UPDATE {} SET {} WHERE {predicate}",
                            qualified(&table.id),
                            sets.join(",")
                        )
                    }
                    RowChange::Delete { original } => format!(
                        "DELETE FROM {} WHERE {}",
                        qualified(&table.id),
                        original_predicate(table, original, &mut params)?
                    ),
                };
                let refs: Vec<&(dyn ToSql + Sync)> =
                    params.iter().map(|s| s as &(dyn ToSql + Sync)).collect();
                let count = tx.execute(&sql, &refs).await.map_err(db_error)?;
                if count != 1 {
                    return Err(Error::Conflict);
                }
                affected += count;
            }
            tx.commit().await.map_err(db_error)?;
            Ok(affected)
        })
    }
    fn execute_sql(&mut self, sql: &str, limit: usize, emit: &mut dyn FnMut(SqlEvent)) {
        let started = Instant::now();
        let result = self.runtime.block_on(async {
            let stream = self.client.simple_query_raw(sql).await.map_err(db_error)?;
            pin_mut!(stream);
            let mut chunk = vec![];
            let mut total = 0;
            while let Some(message) = stream.next().await {
                match message.map_err(db_error)? {
                    SimpleQueryMessage::RowDescription(columns) => {
                        total = 0;
                        emit(SqlEvent::Columns(
                            columns.iter().map(|c| c.name().to_owned()).collect(),
                        ));
                    }
                    SimpleQueryMessage::Row(row) => {
                        if total < limit {
                            chunk.push(
                                (0..row.len())
                                    .map(|i| {
                                        row.get(i)
                                            .map(|s| Value::Text(s.into()))
                                            .unwrap_or(Value::Null)
                                    })
                                    .collect(),
                            );
                        }
                        total += 1;
                        if chunk.len() == 200 {
                            emit(SqlEvent::Rows(std::mem::take(&mut chunk)));
                        }
                    }
                    SimpleQueryMessage::CommandComplete(affected) => {
                        if !chunk.is_empty() {
                            emit(SqlEvent::Rows(std::mem::take(&mut chunk)));
                        }
                        emit(SqlEvent::Complete {
                            affected,
                            truncated: total > limit,
                        });
                        total = 0;
                    }
                    _ => {}
                }
            }
            Ok::<_, Error>(())
        });
        if let Err(e) = result {
            emit(SqlEvent::Error(e));
        }
        emit(SqlEvent::Finished {
            elapsed_ms: started.elapsed().as_millis(),
        });
    }
    fn cancel_handle(&self) -> Arc<dyn CancelQuery> {
        Arc::new(PgCancel {
            token: self.client.cancel_token(),
            runtime: self.runtime.clone(),
            tls: self.tls.clone(),
        })
    }
}
fn bind_input(input: &InputValue, col: &Column, params: &mut Vec<Option<String>>) -> String {
    match input {
        InputValue::Default => "DEFAULT".into(),
        InputValue::Null => {
            params.push(None);
            format!("${}::text::{}", params.len(), col.data_type)
        }
        InputValue::Text(value) => {
            params.push(Some(value.clone()));
            format!("${}::text::{}", params.len(), col.data_type)
        }
    }
}
fn original_predicate(
    table: &Table,
    row: &DataRow,
    params: &mut Vec<Option<String>>,
) -> Result<String> {
    let keys = table.identity_columns();
    if keys.is_empty() || row.values.len() != table.columns.len() {
        return Err(Error::Invalid(
            "Aucune identité de ligne fiable ; utilisez SQL".into(),
        ));
    }
    let mut predicates = vec![];
    for name in keys {
        let i = table
            .column_index(&name)
            .ok_or_else(|| Error::Invalid("Identité périmée".into()))?;
        params.push(row.values[i].text());
        predicates.push(format!(
            "{} IS NOT DISTINCT FROM ${}::text::{}",
            quote(&name),
            params.len(),
            table.columns[i].data_type
        ));
    }
    params.push(
        row.version
            .clone()
            .ok_or_else(|| Error::Invalid("Version de ligne manquante".into()))?
            .into(),
    );
    predicates.push(format!("xmin::text = ${}::text", params.len()));
    Ok(predicates.join(" AND "))
}
fn referential_action(code: &str) -> String {
    match code {
        "a" => "NO ACTION",
        "r" => "RESTRICT",
        "c" => "CASCADE",
        "n" => "SET NULL",
        "d" => "SET DEFAULT",
        _ => "?",
    }
    .into()
}
