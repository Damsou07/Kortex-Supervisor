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
fn f1_opens_the_searchable_keyboard_catalogue() {
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
    harness.key_combination(&[egui::Key::F1]);
    harness.run();
    assert!(harness.query_by_label("Ctrl+W").is_some());
    assert!(harness.query_by_label("Ctrl+G").is_some());
    harness.snapshot("shortcuts");
}

#[test]
fn data_header_types_and_key_badges_keep_sort_and_draft_guards() {
    use kortex_supervisor::grid::{self, DataView};
    let mut table = Table {
        stable_id: "t".into(),
        id: ObjectId::new("main", "documents"),
        columns: vec![
            Column::new("id", "INTEGER", false),
            Column::new("libellé", "character varying(255)", true),
            Column::new("créé_le", "timestamp with time zone", false),
        ],
        primary_key: vec!["id".into()],
        unique_keys: vec![vec!["id".into(), "libellé".into()]],
        ..Table::default()
    };
    table.foreign_keys.push(ForeignKey {
        columns: vec!["id".into()],
        ..ForeignKey::default()
    });
    let mut view = DataView::new(table);
    view.grid.page.rows = vec![DataRow {
        values: vec![
            Value::Integer(1),
            Value::Text("Document de démonstration".into()),
            Value::Text("2026-10-29 12:00:00+01".into()),
        ],
        version: None,
    }];
    let mut initialized = false;
    let mut harness = Harness::builder()
        .with_size([1100.0, 600.0])
        .build_ui_state(
            move |ui, view: &mut DataView| {
                if !initialized {
                    kortex_supervisor::theme::apply(ui.ctx());
                    initialized = true;
                }
                grid::show(ui, view);
            },
            view,
        );
    harness.run();
    harness.snapshot("grid");
    harness.get_by_label("libellé").click();
    harness.run();
    assert_eq!(harness.state().query.sort[0].column, "libellé");
    harness.state_mut().grid.inserted.push(Default::default());
    harness.run();
    harness
        .get_by_role_and_label(egui::Role::Button, "id")
        .click();
    harness.run();
    assert_eq!(harness.state().query.sort[0].column, "libellé");
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
    view.prepare_geometry(&catalog, &layout);
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
