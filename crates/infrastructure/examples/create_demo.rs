fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = std::env::args()
        .nth(1)
        .ok_or("Usage: create_demo <nouveau-fichier.db>")?;
    if std::path::Path::new(&path).exists() {
        return Err("Le fichier existe déjà".into());
    }
    let db = rusqlite::Connection::open(path)?;
    db.execute_batch("PRAGMA foreign_keys=ON;
        CREATE TABLE clients(id INTEGER PRIMARY KEY, nom TEXT NOT NULL, email TEXT UNIQUE NOT NULL, actif INTEGER NOT NULL DEFAULT 1);
        CREATE TABLE projets(id INTEGER PRIMARY KEY, client_id INTEGER NOT NULL REFERENCES clients(id), nom TEXT NOT NULL, budget NUMERIC CHECK(budget>=0), statut TEXT DEFAULT 'En cours');
        CREATE TABLE collaborateurs(id INTEGER PRIMARY KEY, nom TEXT NOT NULL, email TEXT NOT NULL UNIQUE);
        CREATE TABLE taches(id INTEGER PRIMARY KEY, projet_id INTEGER NOT NULL REFERENCES projets(id), responsable_id INTEGER REFERENCES collaborateurs(id), titre TEXT NOT NULL, terminee INTEGER NOT NULL DEFAULT 0);
        CREATE INDEX idx_projets_client ON projets(client_id);
        CREATE INDEX idx_taches_projet ON taches(projet_id);
        INSERT INTO clients VALUES(1,'Atelier Horizon','contact@example.test',1),(2,'Studio Nova','studio@example.test',1);
        INSERT INTO projets VALUES(1,1,'Portail client',15000,'En cours'),(2,2,'Catalogue',8500,'Planifié');
        INSERT INTO collaborateurs VALUES(1,'Camille','camille@example.test'),(2,'Alex','alex@example.test');
        INSERT INTO taches VALUES(1,1,1,'Concevoir le schéma',1),(2,1,2,'Développer les vues',0),(3,2,NULL,'Préparer le catalogue',0);")?;
    Ok(())
}
