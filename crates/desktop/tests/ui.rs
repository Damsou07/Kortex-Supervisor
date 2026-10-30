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
fn branding_and_toolbar_fit_the_minimum_window_size() {
    let mut app = KortexApp::for_test();
    let mut initialized = false;
    let mut harness = Harness::builder()
        .with_size([1000.0, 650.0])
        .build_ui(move |ui| {
            if !initialized {
                kortex_supervisor::theme::apply(ui.ctx());
                initialized = true;
            }
            app.render(ui);
        });
    harness.run();
    let logo = harness.get_by_label("Logo Kortex Supervisor").rect();
    assert_eq!(logo.size(), egui::vec2(64.0, 64.0));
    for label in [
        "KORTEX",
        "SUPERVISOR",
        "Nouvelle connexion",
        "Ouvrir un fichier SQLite",
        "Paramètres",
        "Réinitialiser les panneaux",
    ] {
        let bounds = harness.get_by_label(label).rect();
        assert!(
            egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1000.0, 100.0))
                .contains_rect(bounds),
            "header control {label} is clipped: {bounds:?}"
        );
        assert!(!logo.intersects(bounds), "logo overlaps {label}");
    }
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
    harness.get_by_label("Thème MCD").click();
    harness.run();
    harness.get_by_label("Bleu nuit").click();
    harness.run();
    harness.snapshot("mcd_midnight");
    for (label, snapshot) in [
        ("Obsidienne rouge", "mcd_obsidian"),
        ("Émeraude", "mcd_emerald"),
        ("Améthyste", "mcd_amethyst"),
        ("Contraste élevé", "mcd_high_contrast"),
    ] {
        harness.get_by_label("Thème MCD").click();
        harness.run();
        harness.get_by_label(label).click();
        harness.run();
        harness.snapshot(snapshot);
    }
    harness.get_by_label("Thème MCD").click();
    harness.run();
    harness.get_by_label("Bleu nuit").click();
    harness.run();
    harness.get_by_label("Légende").click();
    harness.run();
    harness.get_by_label("Détaillée").click();
    harness.run();
    assert!(harness.query_by_label("Clé primaire").is_some());
    harness.snapshot("legend_detailed");
}

#[test]
fn legend_cards_preserve_type_and_key_meanings_in_every_palette() {
    use kortex_infrastructure::storage::{Appearance, DiagramTheme, LegendPalette, LegendStyle};
    use kortex_supervisor::{columns, theme};

    let mut snapshot_results = egui_kittest::SnapshotResults::new();
    for (palette, snapshot) in [
        (LegendPalette::FollowDiagram, "legend_cards_follow_mcd"),
        (LegendPalette::Neutral, "legend_cards_neutral"),
        (LegendPalette::HighContrast, "legend_cards_high_contrast"),
    ] {
        let mut initialized = false;
        let mut harness = Harness::builder()
            .with_size([650.0, 800.0])
            .build_ui(move |ui| {
                if !initialized {
                    theme::apply(ui.ctx());
                    theme::set_appearance(
                        ui.ctx(),
                        Appearance {
                            diagram_theme: DiagramTheme::Emerald,
                            legend_style: LegendStyle::Cards,
                            legend_palette: palette,
                        },
                    );
                    initialized = true;
                }
                columns::legend(ui);
            });
        harness.run();
        harness.get_by_label("Légende").click();
        harness.run();
        for label in ["● Nombres", "● Textes"] {
            assert!(harness.query_by_label(label).is_some(), "missing {label}");
        }
        harness.snapshot(snapshot);
        for _ in 0..4 {
            harness.get_by_label("● Nombres").scroll_down();
        }
        harness.run();
        for label in ["Clé primaire", "Clé étrangère"] {
            assert!(harness.query_by_label(label).is_some(), "missing {label}");
        }
        harness.snapshot(format!("{snapshot}_keys"));
        snapshot_results.extend_harness(&mut harness);
    }
    snapshot_results.into_result().unwrap();
}
