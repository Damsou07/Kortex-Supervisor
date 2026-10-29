use kortex_application::DatabaseBackend;
use kortex_domain::*;
use kortex_infrastructure::postgres::PostgresBackend;
use std::collections::BTreeMap;

// Explicitly invoked by scripts/Test-Postgres.ps1 and CI against a disposable database.
#[test]
#[ignore = "requires a disposable PostgreSQL instance; run scripts/Test-Postgres.ps1"]
fn postgres_end_to_end() {
    let port: u16 = std::env::var("KORTEX_TEST_PG_PORT")
        .expect("test port required")
        .parse()
        .unwrap();
    let engine = Engine::Postgres {
        host: "127.0.0.1".into(),
        port,
        database: "postgres".into(),
        user: "kortex".into(),
        tls: false,
        ca_file: String::new(),
    };
    let mut db = PostgresBackend::connect(&engine, "").unwrap();
    let schema = format!("kortex_test_{}", std::process::id());
    let mut events = vec![];
    db.execute_sql(&format!("CREATE SCHEMA {schema}; CREATE TABLE {schema}.customers(id bigint PRIMARY KEY, name text NOT NULL UNIQUE, meta jsonb, external_id uuid); CREATE TABLE {schema}.orders(id bigint PRIMARY KEY, customer_id bigint REFERENCES {schema}.customers(id), amount numeric CHECK(amount >= 0)); INSERT INTO {schema}.customers VALUES(1,'Ada','{{\"a\":1}}','12345678-1234-1234-1234-123456789abc'),(2,'Grace',NULL,NULL); INSERT INTO {schema}.orders VALUES(1,1,20);"), 100, &mut |e|events.push(e));
    assert!(
        !events.iter().any(|e| matches!(e, SqlEvent::Error(_))),
        "{events:?}"
    );
    let catalog = db.catalog().unwrap();
    let table = catalog
        .tables
        .iter()
        .find(|t| t.id.schema == schema && t.id.name == "customers")
        .unwrap();
    assert_eq!(table.identity_columns(), vec!["id"]);
    let orders = catalog
        .tables
        .iter()
        .find(|t| t.id.schema == schema && t.id.name == "orders")
        .unwrap();
    assert_eq!(orders.foreign_keys[0].target, table.id);
    events.clear();
    db.execute_sql(&format!("CREATE TABLE {schema}.parent(a int,b int,PRIMARY KEY(a,b)); CREATE TABLE {schema}.child(id int PRIMARY KEY,a int,b int,CONSTRAINT reference_pair FOREIGN KEY(a,b) REFERENCES {schema}.parent(a,b) MATCH FULL);"),10,&mut |e|events.push(e));
    assert!(!events.iter().any(|e| matches!(e, SqlEvent::Error(_))));
    let composite = db
        .catalog()
        .unwrap()
        .tables
        .into_iter()
        .find(|t| t.id.schema == schema && t.id.name == "child")
        .unwrap();
    assert_eq!(composite.foreign_keys[0].columns, vec!["a", "b"]);
    assert_eq!(composite.foreign_keys[0].target_columns, vec!["a", "b"]);
    assert!(composite.foreign_keys[0].match_full);
    let query = PageQuery {
        limit: 1,
        ..PageQuery::default()
    };
    let page = db.page(table, &query).unwrap();
    assert!(page.has_more);
    let next = db
        .page(
            table,
            &PageQuery {
                after: cursor_for(table, &query, &page.rows[0]),
                ..query
            },
        )
        .unwrap();
    assert_eq!(next.rows[0].values[1].display(), "Grace");
    let changes = ChangeSet {
        changes: vec![
            RowChange::Update {
                original: page.rows[0].clone(),
                values: BTreeMap::from([("name".into(), InputValue::Text("updated".into()))]),
            },
            RowChange::Insert {
                values: BTreeMap::from([
                    ("id".into(), InputValue::Text("3".into())),
                    ("name".into(), InputValue::Text("Grace".into())),
                ]),
            },
        ],
    };
    assert!(db.apply(table, &changes).is_err());
    assert_eq!(
        db.page(table, &PageQuery::default()).unwrap().rows[0].values[1].display(),
        "Ada"
    );
    let mut external = PostgresBackend::connect(&engine, "").unwrap();
    external.execute_sql(
        &format!("UPDATE {schema}.customers SET name='external' WHERE id=1"),
        100,
        &mut |_| {},
    );
    assert_eq!(
        db.apply(
            table,
            &ChangeSet {
                changes: vec![changes.changes[0].clone()]
            }
        ),
        Err(Error::Conflict)
    );
    let cancellation = db.cancel_handle();
    let cancel_thread = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(150));
        cancellation.cancel();
    });
    events.clear();
    db.execute_sql("SELECT pg_sleep(10)", 10, &mut |e| events.push(e));
    cancel_thread.join().unwrap();
    assert!(
        events
            .iter()
            .any(|e| matches!(e, SqlEvent::Error(Error::Cancelled)))
    );
    let role = format!("kortex_reader_{}", std::process::id());
    events.clear();
    db.execute_sql(&format!("CREATE ROLE {role}; GRANT USAGE ON SCHEMA {schema} TO {role}; GRANT SELECT ON {schema}.customers TO {role}; SET ROLE {role};"),10,&mut |e|events.push(e));
    assert!(
        !events.iter().any(|e| matches!(e, SqlEvent::Error(_))),
        "{events:?}"
    );
    let fresh = db.page(table, &PageQuery::default()).unwrap();
    assert!(
        matches!(db.apply(table,&ChangeSet{changes:vec![RowChange::Delete{original:fresh.rows[0].clone()}]}),Err(Error::Database(message)) if message.contains("42501"))
    );
    events.clear();
    db.execute_sql("RESET ROLE", 10, &mut |e| events.push(e));
    db.execute_sql(
        &format!("BEGIN; INSERT INTO {schema}.customers(id,name) VALUES(90,'temporary');"),
        10,
        &mut |e| events.push(e),
    );
    db.execute_sql(
        &format!("SELECT name FROM {schema}.customers WHERE id=90; ROLLBACK;"),
        10,
        &mut |e| events.push(e),
    );
    assert!(
        !events.iter().any(|e| matches!(e, SqlEvent::Error(_))),
        "{events:?}"
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(e,SqlEvent::Rows(rows) if rows[0][0].display()=="temporary"))
    );
    events.clear();
    db.execute_sql(
        &format!("DROP OWNED BY {role}; DROP ROLE {role}; DROP SCHEMA {schema} CASCADE"),
        100,
        &mut |e| events.push(e),
    );
    assert!(
        !events.iter().any(|e| matches!(e, SqlEvent::Error(_))),
        "{events:?}"
    );
}
