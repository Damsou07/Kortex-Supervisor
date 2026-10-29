use kortex_application::{CancelQuery, DatabaseBackend};
use kortex_domain::*;
use rusqlite::fallible_iterator::FallibleIterator;
use rusqlite::{
    Connection, OpenFlags,
    types::{Value as SqlValue, ValueRef},
};
use std::{
    collections::BTreeMap,
    sync::Arc,
    time::{Duration, Instant},
};

pub struct SqliteBackend {
    connection: Connection,
}
struct Interrupt(rusqlite::InterruptHandle);
impl CancelQuery for Interrupt {
    fn cancel(&self) {
        self.0.interrupt();
    }
}
fn db_error(e: rusqlite::Error) -> Error {
    if matches!(&e, rusqlite::Error::SqliteFailure(code, _) if code.code == rusqlite::ErrorCode::OperationInterrupted)
    {
        Error::Cancelled
    } else {
        Error::Database(e.to_string())
    }
}
fn value(v: ValueRef<'_>) -> Value {
    match v {
        ValueRef::Null => Value::Null,
        ValueRef::Integer(v) => Value::Integer(v),
        ValueRef::Real(v) => Value::Real(v),
        ValueRef::Text(v) => Value::Text(String::from_utf8_lossy(v).into_owned()),
        ValueRef::Blob(v) => Value::Binary(v.to_vec()),
    }
}
fn sql_value(v: &Value) -> SqlValue {
    match v {
        Value::Null => SqlValue::Null,
        Value::Integer(v) => SqlValue::Integer(*v),
        Value::Real(v) => SqlValue::Real(*v),
        Value::Text(v) => SqlValue::Text(v.clone()),
        Value::Binary(v) => SqlValue::Blob(v.clone()),
    }
}
fn input_value(input: &InputValue, col: &Column) -> Result<SqlValue> {
    match input {
        InputValue::Null => Ok(SqlValue::Null),
        InputValue::Default => Err(Error::Invalid("DEFAULT doit être traité séparément".into())),
        InputValue::Text(v) => {
            let ty = col.data_type.to_uppercase();
            if ty.contains("BLOB") {
                if v.len() % 2 != 0 || !v.is_ascii() {
                    return Err(Error::Invalid(
                        "Un BLOB doit être une chaîne hexadécimale paire".into(),
                    ));
                }
                let bytes: std::result::Result<Vec<_>, _> = (0..v.len())
                    .step_by(2)
                    .map(|i| u8::from_str_radix(&v[i..i + 2], 16))
                    .collect();
                return bytes
                    .map(SqlValue::Blob)
                    .map_err(|_| Error::Invalid("BLOB hexadécimal invalide".into()));
            }
            // SQLite applies declared affinity itself; text preserves decimal precision and dynamic typing.
            Ok(SqlValue::Text(v.clone()))
        }
    }
}
impl SqliteBackend {
    pub fn open(path: &str) -> Result<Self> {
        let connection = Connection::open_with_flags(
            path,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )
        .map_err(db_error)?;
        connection
            .busy_timeout(Duration::from_secs(3))
            .map_err(db_error)?;
        connection
            .execute_batch("PRAGMA foreign_keys=ON;")
            .map_err(db_error)?;
        connection
            .query_row("SELECT count(*) FROM sqlite_schema", [], |_| Ok(()))
            .map_err(db_error)?;
        Ok(Self { connection })
    }
}
impl DatabaseBackend for SqliteBackend {
    fn catalog(&mut self) -> Result<Catalog> {
        let mut stmt = self.connection.prepare("SELECT name,type,coalesce(sql,'') FROM sqlite_schema WHERE type IN ('table','view') AND name NOT LIKE 'sqlite_%' ORDER BY name").map_err(db_error)?;
        let objects = stmt
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                ))
            })
            .map_err(db_error)?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(db_error)?;
        let mut catalog = Catalog {
            server_version: format!("SQLite {}", rusqlite::version()),
            databases: vec!["main".into()],
            ..Catalog::default()
        };
        for (name, kind, definition) in objects {
            let id = ObjectId::new("main", &name);
            let mut table = Table {
                stable_id: id.key(),
                id,
                is_view: kind == "view",
                definition,
                ..Table::default()
            };
            let mut columns = self
                .connection
                .prepare(&format!("PRAGMA main.table_xinfo({})", quote(&name)))
                .map_err(db_error)?;
            let details = columns
                .query_map([], |r| {
                    Ok((
                        Column {
                            name: r.get(1)?,
                            data_type: r.get(2)?,
                            nullable: r.get::<_, i32>(3)? == 0,
                            default: r.get(4)?,
                            generated: r.get::<_, i32>(6)? != 0,
                            comment: String::new(),
                        },
                        r.get::<_, i32>(5)?,
                    ))
                })
                .map_err(db_error)?
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(db_error)?;
            let mut primary: Vec<_> = details
                .iter()
                .filter(|(_, p)| *p > 0)
                .map(|(c, p)| (*p, c.name.clone()))
                .collect();
            primary.sort_by_key(|(p, _)| *p);
            table.primary_key = primary.into_iter().map(|(_, n)| n).collect();
            let (without_rowid, strict): (bool, bool) = self
                .connection
                .query_row(
                    "SELECT wr, strict FROM pragma_table_list WHERE schema='main' AND name=?1",
                    [&name],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .map_err(db_error)?;
            let has_pk_index: bool = self
                .connection
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM pragma_index_list(?1) WHERE origin='pk')",
                    [&name],
                    |r| r.get(0),
                )
                .map_err(db_error)?;
            table.columns = details
                .into_iter()
                .map(|(mut c, pk)| {
                    if pk > 0
                        && (without_rowid
                            || strict
                            || (!has_pk_index
                                && table.primary_key.len() == 1
                                && c.data_type.eq_ignore_ascii_case("INTEGER")))
                    {
                        c.nullable = false;
                    }
                    c
                })
                .collect();
            let mut indexes = self
                .connection
                .prepare(&format!("PRAGMA main.index_list({})", quote(&name)))
                .map_err(db_error)?;
            let index_list = indexes
                .query_map([], |r| {
                    Ok((
                        r.get::<_, String>(1)?,
                        r.get::<_, i32>(2)? != 0,
                        r.get::<_, i32>(4)? != 0,
                    ))
                })
                .map_err(db_error)?
                .collect::<std::result::Result<Vec<_>, _>>()
                .map_err(db_error)?;
            for (index_name, unique, partial) in index_list {
                let mut index_info = self
                    .connection
                    .prepare(&format!("PRAGMA main.index_info({})", quote(&index_name)))
                    .map_err(db_error)?;
                let names = index_info
                    .query_map([], |r| r.get::<_, Option<String>>(2))
                    .map_err(db_error)?
                    .collect::<std::result::Result<Vec<_>, _>>()
                    .map_err(db_error)?;
                let pure_columns = names.iter().all(Option::is_some);
                let cols = names.into_iter().flatten().collect::<Vec<_>>();
                if unique && !partial && pure_columns {
                    table.unique_keys.push(cols.clone());
                }
                let definition = self
                    .connection
                    .query_row(
                        "SELECT coalesce(sql,'Index automatique') FROM sqlite_schema WHERE name=?1",
                        [&index_name],
                        |r| r.get(0),
                    )
                    .unwrap_or_default();
                table.indexes.push(Index {
                    name: index_name,
                    columns: cols,
                    unique,
                    partial,
                    definition,
                });
            }
            let mut fks = self
                .connection
                .prepare(&format!("PRAGMA main.foreign_key_list({})", quote(&name)))
                .map_err(db_error)?;
            let mut rows = fks.query([]).map_err(db_error)?;
            let mut relations = BTreeMap::<i64, ForeignKey>::new();
            while let Some(r) = rows.next().map_err(db_error)? {
                let fk_id: i64 = r.get(0).map_err(db_error)?;
                let fk = relations.entry(fk_id).or_insert_with(|| ForeignKey {
                    id: format!("{}:{fk_id}", table.stable_id),
                    name: format!("fk_{name}_{fk_id}"),
                    source: table.id.clone(),
                    ..ForeignKey::default()
                });
                fk.target = ObjectId::new("main", &r.get::<_, String>(2).map_err(db_error)?);
                fk.columns.push(r.get(3).map_err(db_error)?);
                fk.target_columns.push(
                    r.get::<_, Option<String>>(4)
                        .map_err(db_error)?
                        .unwrap_or_default(),
                );
                fk.on_update = r.get(5).map_err(db_error)?;
                fk.on_delete = r.get(6).map_err(db_error)?;
            }
            table.foreign_keys = relations.into_values().collect();
            if !table.primary_key.is_empty() {
                table.constraints.push(Constraint {
                    name: "PRIMARY KEY".into(),
                    kind: "PK".into(),
                    definition: table.primary_key.join(", "),
                });
            }
            for key in &table.unique_keys {
                table.constraints.push(Constraint {
                    name: "UNIQUE".into(),
                    kind: "UQ".into(),
                    definition: key.join(", "),
                });
            }
            for fk in &table.foreign_keys {
                table.constraints.push(Constraint {
                    name: fk.name.clone(),
                    kind: "FK".into(),
                    definition: format!(
                        "{} → {} ({}) · DELETE {} · UPDATE {}",
                        fk.columns.join(", "),
                        fk.target.label(),
                        fk.target_columns.join(", "),
                        fk.on_delete,
                        fk.on_update
                    ),
                });
            }
            catalog.tables.push(table);
        }
        let pks: BTreeMap<_, _> = catalog
            .tables
            .iter()
            .map(|t| (t.id.clone(), t.primary_key.clone()))
            .collect();
        for table in &mut catalog.tables {
            for fk in &mut table.foreign_keys {
                if fk.target_columns.iter().all(String::is_empty) {
                    fk.target_columns = pks.get(&fk.target).cloned().unwrap_or_default();
                }
            }
        }
        Ok(catalog)
    }
    fn page(&mut self, table: &Table, query: &PageQuery) -> Result<DataPage> {
        let built = build_page_query(table, query, Dialect::Sqlite)?;
        let mut stmt = self.connection.prepare(&built.sql).map_err(db_error)?;
        let mut cursor = stmt
            .query(rusqlite::params_from_iter(
                built.params.iter().map(sql_value),
            ))
            .map_err(db_error)?;
        let mut rows = vec![];
        while let Some(row) = cursor.next().map_err(db_error)? {
            let values = (0..table.columns.len())
                .map(|i| row.get_ref(i).map(value))
                .collect::<std::result::Result<_, _>>()
                .map_err(db_error)?;
            rows.push(DataRow {
                values,
                version: None,
            });
        }
        let has_more = rows.len() > query.limit.clamp(1, 2000);
        rows.truncate(query.limit.clamp(1, 2000));
        Ok(DataPage { rows, has_more })
    }
    fn apply(&mut self, table: &Table, changes: &ChangeSet) -> Result<u64> {
        if table.is_view {
            return Err(Error::Invalid(
                "Modifiez les vues depuis l’éditeur SQL".into(),
            ));
        }
        let tx = self
            .connection
            .transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)
            .map_err(db_error)?;
        let mut affected = 0;
        for change in &changes.changes {
            let mut params: Vec<SqlValue> = vec![];
            let sql = match change {
                RowChange::Insert { values } => {
                    let mut cols = vec![];
                    let mut bindings = vec![];
                    for (name, input) in values {
                        let c = column(table, name)?;
                        if c.generated || matches!(input, InputValue::Default) {
                            continue;
                        }
                        params.push(input_value(input, c)?);
                        cols.push(quote(name));
                        bindings.push(format!("?{}", params.len()));
                    }
                    if cols.is_empty() {
                        format!("INSERT INTO {} DEFAULT VALUES", qualified(&table.id))
                    } else {
                        format!(
                            "INSERT INTO {} ({}) VALUES ({})",
                            qualified(&table.id),
                            cols.join(","),
                            bindings.join(",")
                        )
                    }
                }
                RowChange::Update { original, values } => {
                    let mut sets = vec![];
                    for (name, input) in values {
                        let c = column(table, name)?;
                        if c.generated {
                            return Err(Error::Invalid("Colonne générée non modifiable".into()));
                        }
                        if matches!(input, InputValue::Default) {
                            sets.push(format!(
                                "{} = {}",
                                quote(name),
                                c.default.as_deref().unwrap_or("NULL")
                            ));
                        } else {
                            params.push(input_value(input, c)?);
                            sets.push(format!("{} = ?{}", quote(name), params.len()));
                        }
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
                RowChange::Delete { original } => {
                    let predicate = original_predicate(table, original, &mut params)?;
                    format!("DELETE FROM {} WHERE {predicate}", qualified(&table.id))
                }
            };
            let count = tx
                .execute(&sql, rusqlite::params_from_iter(params))
                .map_err(db_error)?;
            if count != 1 {
                return Err(Error::Conflict);
            }
            affected += count as u64;
        }
        tx.commit().map_err(db_error)?;
        Ok(affected)
    }
    fn execute_sql(&mut self, sql: &str, limit: usize, emit: &mut dyn FnMut(SqlEvent)) {
        let started = Instant::now();
        let outcome = (|| -> Result<()> {
            let mut batch = rusqlite::Batch::new(&self.connection, sql);
            while let Some(mut statement) = batch.next().map_err(db_error)? {
                let count = statement.column_count();
                if count == 0 {
                    let affected = statement.execute([]).map_err(db_error)? as u64;
                    emit(SqlEvent::Columns(vec![]));
                    emit(SqlEvent::Complete {
                        affected,
                        truncated: false,
                    });
                } else {
                    emit(SqlEvent::Columns(
                        statement.column_names().map(str::to_owned).collect(),
                    ));
                    let mut rows = statement.query([]).map_err(db_error)?;
                    let mut chunk = vec![];
                    let mut total = 0;
                    while let Some(row) = rows.next().map_err(db_error)? {
                        if total < limit {
                            chunk.push(
                                (0..count)
                                    .map(|i| row.get_ref(i).map(value))
                                    .collect::<std::result::Result<_, _>>()
                                    .map_err(db_error)?,
                            );
                        }
                        total += 1;
                        if chunk.len() == 200 {
                            emit(SqlEvent::Rows(std::mem::take(&mut chunk)));
                        }
                    }
                    if !chunk.is_empty() {
                        emit(SqlEvent::Rows(chunk));
                    }
                    emit(SqlEvent::Complete {
                        affected: total as u64,
                        truncated: total > limit,
                    });
                }
            }
            Ok(())
        })();
        if let Err(e) = outcome {
            emit(SqlEvent::Error(e));
        }
        emit(SqlEvent::Finished {
            elapsed_ms: started.elapsed().as_millis(),
        });
    }
    fn cancel_handle(&self) -> Arc<dyn CancelQuery> {
        Arc::new(Interrupt(self.connection.get_interrupt_handle()))
    }
}
fn original_predicate(table: &Table, row: &DataRow, params: &mut Vec<SqlValue>) -> Result<String> {
    if table.identity_columns().is_empty() || row.values.len() != table.columns.len() {
        return Err(Error::Invalid(
            "Aucune identité de ligne fiable ; utilisez SQL".into(),
        ));
    }
    Ok(table
        .columns
        .iter()
        .zip(&row.values)
        .map(|(c, v)| {
            params.push(sql_value(v));
            format!(
                "({0} COLLATE BINARY IS ?{1} AND typeof({0}) = typeof(?{1}))",
                quote(&c.name),
                params.len()
            )
        })
        .collect::<Vec<_>>()
        .join(" AND "))
}
