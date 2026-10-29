use kortex_application::DatabaseBackend;
use kortex_domain::*;
use kortex_infrastructure::sqlite::SqliteBackend;
use std::time::Instant;

#[test]
#[ignore = "performance measurement; run in release mode"]
fn million_rows_remain_paginated() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("million.db");
    let db = rusqlite::Connection::open(&path).unwrap();
    db.execute_batch("CREATE TABLE items(id INTEGER PRIMARY KEY, value TEXT); WITH RECURSIVE n(x) AS (SELECT 1 UNION ALL SELECT x+1 FROM n WHERE x<1000000) INSERT INTO items SELECT x,'row '||x FROM n;").unwrap();
    drop(db);
    let mut backend = SqliteBackend::open(path.to_str().unwrap()).unwrap();
    let table = backend.catalog().unwrap().tables.remove(0);
    let start = Instant::now();
    let page = backend
        .page(
            &table,
            &PageQuery {
                after: Some(vec![Value::Integer(900000)]),
                ..PageQuery::default()
            },
        )
        .unwrap();
    println!(
        "SQLite 1M rows, page at 900000: {:.2}ms, {} resident rows",
        start.elapsed().as_secs_f64() * 1000.0,
        page.rows.len()
    );
    assert_eq!(page.rows.len(), 200);
    assert!(page.has_more);
    assert_eq!(page.rows[0].values[0], Value::Integer(900001));
    assert!(start.elapsed().as_secs_f64() < 2.0);
}
