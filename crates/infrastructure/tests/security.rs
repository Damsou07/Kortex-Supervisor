use kortex_domain::*;
use kortex_infrastructure::postgres::PostgresBackend;

fn remote(tls: bool, ca_file: String) -> Engine {
    Engine::Postgres {
        host: "db.example.test".into(),
        port: 5432,
        database: "test".into(),
        user: "test".into(),
        tls,
        ca_file,
    }
}
#[test]
fn plaintext_remote_connection_is_rejected_before_network_access() {
    assert!(
        matches!(PostgresBackend::connect(&remote(false,String::new()), ""),Err(Error::Invalid(message)) if message.contains("TLS"))
    );
}
#[test]
fn empty_custom_ca_is_rejected_before_network_access() {
    let file = tempfile::NamedTempFile::new().unwrap();
    assert!(
        matches!(PostgresBackend::connect(&remote(true,file.path().display().to_string()), ""),Err(Error::Invalid(message)) if message.contains("certificat"))
    );
}
