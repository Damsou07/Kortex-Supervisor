use kortex_application::DatabaseBackend;
use kortex_domain::*;
use kortex_infrastructure::postgres::PostgresBackend;

#[test]
#[ignore = "requires disposable TLS server; scripts/Test-Postgres.ps1 -Tls"]
fn tls_checks_custom_authority_and_hostname() {
    let port = std::env::var("KORTEX_TEST_PG_PORT")
        .unwrap()
        .parse()
        .unwrap();
    let ca = std::env::var("KORTEX_TEST_CA").unwrap();
    let engine = |host: &str, ca_file: String| Engine::Postgres {
        host: host.into(),
        port,
        database: "postgres".into(),
        user: "kortex".into(),
        tls: true,
        ca_file,
    };
    let mut db = PostgresBackend::connect(&engine("localhost", ca.clone()), "")
        .expect("test CA should be trusted");
    assert!(db.catalog().is_ok());
    // The server certificate names localhost only, not 127.0.0.1.
    assert!(PostgresBackend::connect(&engine("127.0.0.1", ca), "").is_err());
    assert!(PostgresBackend::connect(&engine("localhost", String::new()), "").is_err());
}
