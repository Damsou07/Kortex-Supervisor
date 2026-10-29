use egui_kittest::Harness;
use kortex_domain::*;
use kortex_supervisor::mcd::{self, DiagramView};

struct State {
    catalog: Catalog,
    layout: Diagram,
    history: LayoutHistory,
    view: DiagramView,
}
fn contains(shape: &egui::epaint::Shape, needle: &str) -> bool {
    match shape {
        egui::epaint::Shape::Text(t) => t.galley.job.text.contains(needle),
        egui::epaint::Shape::Vec(v) => v.iter().any(|shape| contains(shape, needle)),
        _ => false,
    }
}

#[test]
fn partial_view_draws_incoming_and_outgoing_names() {
    let mut catalog = Catalog::default();
    for name in ["selected", "incoming", "outgoing"] {
        catalog.tables.push(Table {
            stable_id: name.into(),
            id: ObjectId::new("main", name),
            columns: vec![Column::new("id", "integer", false)],
            ..Table::default()
        });
    }
    for (source, target, id) in [(1, 0, "in"), (0, 2, "out")] {
        let fk = ForeignKey {
            id: id.into(),
            source: catalog.tables[source].id.clone(),
            target: catalog.tables[target].id.clone(),
            columns: vec!["id".into()],
            target_columns: vec!["id".into()],
            ..ForeignKey::default()
        };
        catalog.tables[source].foreign_keys.push(fk);
    }
    let mut layout = Diagram::default();
    for (id, x) in [("incoming", 0.0), ("selected", 350.0), ("outgoing", 700.0)] {
        layout.positions.insert(id.into(), Point { x, y: 120.0 });
    }
    let mut view = DiagramView::default();
    view.fit = false;
    view.zoom = 1.0;
    view.prepare_geometry(&catalog, &layout);
    let mut initialized = false;
    let mut harness = Harness::builder()
        .with_size([1200.0, 700.0])
        .build_ui_state(
            move |ui, state: &mut State| {
                if !initialized {
                    kortex_supervisor::theme::apply(ui.ctx());
                    initialized = true;
                }
                mcd::show(
                    ui,
                    &state.catalog,
                    &mut state.layout,
                    &mut state.history,
                    &mut state.view,
                    Some(&["selected".into()]),
                );
            },
            State {
                catalog,
                layout,
                history: LayoutHistory::default(),
                view,
            },
        );
    harness.run();
    for name in ["incoming", "outgoing"] {
        assert!(
            harness
                .output()
                .shapes
                .iter()
                .any(|shape| contains(&shape.shape, name)),
            "missing {name}"
        );
    }
}

#[test]
fn crowded_cardinalities_are_available_in_a_scrollable_list() {
    use egui_kittest::kittest::Queryable;
    let source = ObjectId::new("main", "source_table");
    let target = ObjectId::new("main", "target_table");
    let mut catalog = Catalog {
        tables: vec![
            Table {
                stable_id: "source".into(),
                id: source.clone(),
                columns: vec![Column::new("id", "integer", false)],
                ..Table::default()
            },
            Table {
                stable_id: "target".into(),
                id: target.clone(),
                columns: vec![Column::new("id", "integer", false)],
                ..Table::default()
            },
        ],
        ..Catalog::default()
    };
    for i in 0..70 {
        catalog.tables[0].foreign_keys.push(ForeignKey {
            id: format!("fk_{i}"),
            name: format!("fk_{i}"),
            source: source.clone(),
            target: target.clone(),
            columns: vec!["id".into()],
            target_columns: vec!["id".into()],
            ..ForeignKey::default()
        });
    }
    let mut layout = Diagram::default();
    layout
        .positions
        .insert("source".into(), Point { x: 100.0, y: 10.0 });
    layout
        .positions
        .insert("target".into(), Point { x: 450.0, y: 10.0 });
    for i in 0..70 {
        layout.routes.insert(
            format!("fk_{i}"),
            Route {
                source_y: Some(48.0),
                ..Route::default()
            },
        );
    }
    let mut view = DiagramView::default();
    view.fit = false;
    view.zoom = 1.0;
    let mut initialized = false;
    let mut harness = Harness::builder().with_size([600.0, 350.0]).build_ui_state(
        move |ui, state: &mut State| {
            if !initialized {
                kortex_supervisor::theme::apply(ui.ctx());
                initialized = true;
            }
            mcd::show(
                ui,
                &state.catalog,
                &mut state.layout,
                &mut state.history,
                &mut state.view,
                Some(&["source".into()]),
            );
        },
        State {
            catalog,
            layout,
            history: LayoutHistory::default(),
            view,
        },
    );
    harness.run();
    harness.get_by_label("Cardinalités").click();
    harness.run();
    assert!(
        harness
            .output()
            .shapes
            .iter()
            .any(|s| contains(&s.shape, "fk_")),
        "overflow list is empty"
    );
    assert!(
        harness
            .output()
            .shapes
            .iter()
            .any(|s| contains(&s.shape, "source_table"))
    );
    assert!(
        harness
            .output()
            .shapes
            .iter()
            .any(|s| contains(&s.shape, "target_table"))
    );
}
