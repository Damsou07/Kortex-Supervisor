use egui_kittest::{Harness, kittest::Queryable};
use kortex_domain::*;
use kortex_supervisor::app::KortexApp;

#[test]
fn empty_workspace_exposes_connection_and_workspace_controls() {
    let mut app = KortexApp::for_test();
    let mut initialized = false;
    let mut harness = Harness::builder()
        .with_size([1500.0, 900.0])
        .build_ui(move |ui| {
            if !initialized {
                kortex_supervisor::theme::apply(ui.ctx());
                initialized = true;
            }
            app.render(ui);
        });
    harness.run();
    assert!(harness.query_by_label("Nouvelle connexion").is_some());
    assert!(harness.query_by_label("Ouvrir un fichier SQLite").is_some());
    harness.snapshot("workspace");
}

#[test]
fn mcd_renders_keys_constraints_and_relations_without_mutating_catalog() {
    let source = Table {
        id: ObjectId::new("main", "commandes"),
        stable_id: "1".into(),
        columns: vec![
            Column::new("id", "INTEGER", false),
            Column::new("client_id", "INTEGER", true),
        ],
        primary_key: vec!["id".into()],
        unique_keys: vec![vec!["client_id".into()]],
        foreign_keys: vec![ForeignKey {
            id: "fk".into(),
            source: ObjectId::new("main", "commandes"),
            target: ObjectId::new("main", "clients"),
            columns: vec!["client_id".into()],
            target_columns: vec!["id".into()],
            ..ForeignKey::default()
        }],
        ..Table::default()
    };
    let target = Table {
        id: ObjectId::new("main", "clients"),
        stable_id: "2".into(),
        columns: vec![Column::new("id", "INTEGER", false)],
        primary_key: vec!["id".into()],
        ..Table::default()
    };
    let catalog = Catalog {
        tables: vec![source, target],
        ..Catalog::default()
    };
    let original = catalog.clone();
    let mut layout = Diagram::default();
    layout.reconcile(&catalog.tables);
    let mut history = LayoutHistory::default();
    let mut view = kortex_supervisor::mcd::DiagramView::default();
    let mut initialized = false;
    let mut harness = Harness::builder()
        .with_size([1200.0, 800.0])
        .build_ui(move |ui| {
            if !initialized {
                kortex_supervisor::theme::apply(ui.ctx());
                initialized = true;
            }
            kortex_supervisor::mcd::show(ui, &catalog, &mut layout, &mut history, &mut view, None);
            assert_eq!(catalog, original);
        });
    harness.run();
    assert!(harness.query_by_label("Cadrer").is_some());
    harness.snapshot("mcd");
}
