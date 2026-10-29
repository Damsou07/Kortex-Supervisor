use kortex_domain::*;
use kortex_infrastructure::storage::{self, Preferences};

#[test]
fn atomic_replace_round_trip_and_unknown_version_preserved() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("workspace.json");
    let mut prefs = Preferences::default();
    prefs.profiles.push(Profile {
        id: "test".into(),
        name: "test".into(),
        engine: Engine::Sqlite {
            path: "test.db".into(),
        },
        remember_password: false,
    });
    let mut diagram = Diagram::default();
    diagram
        .positions
        .insert("orders".into(), Point { x: 42.0, y: 64.0 });
    diagram.routes.insert(
        "fk".into(),
        Route {
            points: vec![Point { x: 100.0, y: 30.0 }],
            source_y: Some(80.0),
            target_right: true,
            ..Route::default()
        },
    );
    diagram.groups.push(Group {
        id: 1,
        name: "Commandes".into(),
        members: vec!["orders".into()],
        origin: Point { x: 1.0, y: 2.0 },
        size: Point { x: 600.0, y: 400.0 },
        color: [110, 80, 25, 45],
    });
    prefs.diagrams.insert("test".into(), diagram.clone());
    storage::save(&path, &prefs).unwrap();
    prefs.data_refresh_secs = 2.0;
    storage::save(&path, &prefs).unwrap();
    assert_eq!(storage::load(&path).unwrap().data_refresh_secs, 2.0);
    assert_eq!(storage::load(&path).unwrap().diagrams["test"], diagram);
    prefs.version = 99;
    storage::save(&path, &prefs).unwrap();
    let original = std::fs::read(&path).unwrap();
    assert!(storage::load(&path).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), original);
}
