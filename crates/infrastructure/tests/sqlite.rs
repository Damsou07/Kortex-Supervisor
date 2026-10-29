use kortex_application::DatabaseBackend;
use kortex_domain::*;
use kortex_infrastructure::sqlite::SqliteBackend;
use std::collections::BTreeMap;

fn fixture() -> (tempfile::TempDir, SqliteBackend) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.db");
    let db = rusqlite::Connection::open(&path).unwrap();
    db.execute_batch("PRAGMA foreign_keys=ON; CREATE TABLE customers(id INTEGER PRIMARY KEY, name TEXT NOT NULL UNIQUE); CREATE TABLE orders(id INTEGER PRIMARY KEY, customer_id INTEGER REFERENCES customers(id), amount NUMERIC CHECK(amount >= 0)); INSERT INTO customers VALUES(1,'Ada'),(2,'Grace'); INSERT INTO orders VALUES(1,1,20);").unwrap();
    drop(db);
    let backend = SqliteBackend::open(path.to_str().unwrap()).unwrap();
    (dir, backend)
}

#[test]
fn introspection_and_pagination_round_trip() {
    let (_dir, mut db) = fixture();
    let catalog = db.catalog().unwrap();
    let orders = catalog
        .tables
        .iter()
        .find(|t| t.id.name == "orders")
        .unwrap();
    assert_eq!(orders.primary_key, vec!["id"]);
    assert_eq!(orders.foreign_keys[0].target.name, "customers");
    assert!(orders.definition.contains("CHECK"));
    let customers = catalog
        .tables
        .iter()
        .find(|t| t.id.name == "customers")
        .unwrap();
    let query = PageQuery {
        limit: 1,
        ..PageQuery::default()
    };
    let first = db.page(customers, &query).unwrap();
    assert!(first.has_more);
    let after = cursor_for(customers, &query, &first.rows[0]);
    let next = db.page(customers, &PageQuery { after, ..query }).unwrap();
    assert_eq!(next.rows[0].values[1].display(), "Grace");
}

#[test]
fn apply_is_atomic_and_external_edits_conflict() {
    let (dir, mut db) = fixture();
    let table = db
        .catalog()
        .unwrap()
        .tables
        .into_iter()
        .find(|t| t.id.name == "customers")
        .unwrap();
    let page = db.page(&table, &PageQuery::default()).unwrap();
    let changes = ChangeSet {
        changes: vec![
            RowChange::Update {
                original: page.rows[0].clone(),
                values: BTreeMap::from([("name".into(), InputValue::Text("changed".into()))]),
            },
            RowChange::Insert {
                values: BTreeMap::from([("name".into(), InputValue::Text("Grace".into()))]),
            },
        ],
    };
    assert!(db.apply(&table, &changes).is_err());
    assert_eq!(
        db.page(&table, &PageQuery::default()).unwrap().rows[0].values[1].display(),
        "Ada"
    );
    let external = rusqlite::Connection::open(dir.path().join("test.db")).unwrap();
    external
        .execute("UPDATE customers SET name='external' WHERE id=1", [])
        .unwrap();
    let only_update = ChangeSet {
        changes: vec![changes.changes[0].clone()],
    };
    assert_eq!(db.apply(&table, &only_update), Err(Error::Conflict));
}

#[test]
fn sql_handles_semicolons_in_strings_and_multiple_results() {
    let (_dir, mut db) = fixture();
    let mut events = vec![];
    db.execute_sql("SELECT 'a;b'; SELECT 42;", 100, &mut |event| {
        events.push(event)
    });
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e, SqlEvent::Columns(_)))
            .count(),
        2
    );
    assert!(!events.iter().any(|e| matches!(e, SqlEvent::Error(_))));
}

#[test]
fn unknown_file_is_not_created_by_open() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("absent.db");
    assert!(SqliteBackend::open(path.to_str().unwrap()).is_err());
    assert!(!path.exists());
}

#[test]
fn concurrent_case_change_under_nocase_collation_is_a_conflict() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.db");
    let external = rusqlite::Connection::open(&path).unwrap();
    external.execute_batch("CREATE TABLE names(id INTEGER PRIMARY KEY, name TEXT COLLATE NOCASE); INSERT INTO names VALUES(1,'Ada');").unwrap();
    let mut db = SqliteBackend::open(path.to_str().unwrap()).unwrap();
    let table = db.catalog().unwrap().tables.remove(0);
    let original = db
        .page(&table, &PageQuery::default())
        .unwrap()
        .rows
        .remove(0);
    external
        .execute_batch("UPDATE names SET name='ADA' WHERE id=1")
        .unwrap();
    let changes = ChangeSet {
        changes: vec![RowChange::Delete { original }],
    };
    assert_eq!(db.apply(&table, &changes), Err(Error::Conflict));
}

#[test]
fn integer_primary_key_desc_is_not_a_nonnullable_rowid_alias() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.db");
    let external = rusqlite::Connection::open(&path).unwrap();
    external.execute_batch("CREATE TABLE nullable_key(id INTEGER PRIMARY KEY DESC, name TEXT); INSERT INTO nullable_key VALUES(NULL,'a'),(NULL,'b');").unwrap();
    let mut db = SqliteBackend::open(path.to_str().unwrap()).unwrap();
    assert!(
        db.catalog().unwrap().tables[0]
            .identity_columns()
            .is_empty()
    );
}

#[test]
fn explicit_transaction_survives_executions_and_can_be_rolled_back() {
    let (_dir, mut db) = fixture();
    let mut events = vec![];
    db.execute_sql(
        "BEGIN; INSERT INTO customers VALUES(3,'Katherine');",
        10,
        &mut |e| events.push(e),
    );
    db.execute_sql(
        "SELECT name FROM customers WHERE id=3; ROLLBACK; SELECT count(*) FROM customers",
        10,
        &mut |e| events.push(e),
    );
    assert!(!events.iter().any(|e| matches!(e, SqlEvent::Error(_))));
    assert!(
        events
            .iter()
            .any(|e| matches!(e,SqlEvent::Rows(rows) if rows[0][0].display()=="Katherine"))
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(e,SqlEvent::Rows(rows) if rows[0][0].display()=="2"))
    );
}

#[test]
fn cancellation_interrupts_engine_and_session_remains_usable() {
    let (_dir, mut db) = fixture();
    let cancellation = db.cancel_handle();
    let thread = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(100));
        cancellation.cancel();
    });
    let mut events = vec![];
    db.execute_sql("WITH RECURSIVE n(x) AS (VALUES(0) UNION ALL SELECT x+1 FROM n WHERE x<1000000000) SELECT sum(x) FROM n",10,&mut |e|events.push(e));
    thread.join().unwrap();
    assert!(
        events
            .iter()
            .any(|e| matches!(e, SqlEvent::Error(Error::Cancelled)))
    );
    assert!(db.catalog().is_ok());
}

#[test]
fn cursor_keeps_binary_and_no_affinity_key_types() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("typed.db");
    let external = rusqlite::Connection::open(&path).unwrap();
    external.execute_batch("CREATE TABLE keys(id BLOB PRIMARY KEY NOT NULL); INSERT INTO keys VALUES(1),(2),(x'01'),(x'02');").unwrap();
    let mut db = SqliteBackend::open(path.to_str().unwrap()).unwrap();
    let table = db.catalog().unwrap().tables.remove(0);
    let mut query = PageQuery {
        limit: 1,
        ..PageQuery::default()
    };
    for expected in [
        Value::Integer(1),
        Value::Integer(2),
        Value::Binary(vec![1]),
        Value::Binary(vec![2]),
    ] {
        let page = db.page(&table, &query).unwrap();
        assert_eq!(page.rows[0].values[0], expected);
        query.after = cursor_for(&table, &query, &page.rows[0]);
    }
}
