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
fn cardinality_font_stays_fixed_at_every_zoom() {
    for zoom in [0.25, 1.0, 2.5] {
        let source = ObjectId::new("main", "source");
        let target = ObjectId::new("main", "target");
        let catalog = Catalog {
            tables: vec![
                Table {
                    stable_id: "source".into(),
                    id: source.clone(),
                    columns: vec![Column::new("id", "integer", false)],
                    foreign_keys: vec![ForeignKey {
                        id: "fk".into(),
                        source,
                        target: target.clone(),
                        columns: vec!["id".into()],
                        target_columns: vec!["id".into()],
                        ..ForeignKey::default()
                    }],
                    ..Table::default()
                },
                Table {
                    stable_id: "target".into(),
                    id: target,
                    columns: vec![Column::new("id", "integer", false)],
                    ..Table::default()
                },
            ],
            ..Catalog::default()
        };
        let mut layout = Diagram::default();
        layout
            .positions
            .insert("source".into(), Point { x: 30.0, y: 70.0 });
        layout
            .positions
            .insert("target".into(), Point { x: 650.0, y: 70.0 });
        let mut view = DiagramView::default();
        view.fit = false;
        view.zoom = zoom;
        view.prepare_geometry(&catalog, &layout);
        let mut history = LayoutHistory::default();
        let mut harness = Harness::builder()
            .with_size([1400.0, 900.0])
            .build_ui(move |ui| {
                kortex_supervisor::theme::apply(ui.ctx());
                mcd::show(ui, &catalog, &mut layout, &mut history, &mut view, None);
            });
        harness.run();
        let cardinalities: Vec<_> = harness
            .output()
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text)
                    if ["0..1", "1..1", "0..N"].contains(&text.galley.job.text.as_str()) =>
                {
                    Some(text)
                }
                _ => None,
            })
            .collect();
        assert!(
            !cardinalities.is_empty(),
            "no visible cardinality at zoom {zoom}"
        );
        for text in cardinalities {
            assert_eq!(
                text.galley.job.sections[0].format.font_id.size, 12.0,
                "zoom {zoom}"
            );
        }
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
fn crowded_cardinalities_do_not_expose_the_removed_button() {
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
    assert!(harness.query_by_label("Cardinalités").is_none());
}

#[test]
fn colliding_badges_follow_selected_then_hovered_relation_priority() {
    let source = ObjectId::new("main", "source");
    let target = ObjectId::new("main", "target");
    let mut table = Table {
        stable_id: "source".into(),
        id: source.clone(),
        columns: vec![
            Column::new("id", "integer", false),
            Column::new("unique_id", "integer", false),
        ],
        unique_keys: vec![vec!["unique_id".into()]],
        ..Table::default()
    };
    for (id, column) in [("a", "id"), ("z", "unique_id")] {
        table.foreign_keys.push(ForeignKey {
            id: id.into(),
            source: source.clone(),
            target: target.clone(),
            columns: vec![column.into()],
            target_columns: vec!["id".into()],
            ..ForeignKey::default()
        });
    }
    let catalog = Catalog {
        tables: vec![
            table,
            Table {
                stable_id: "target".into(),
                id: target,
                columns: vec![Column::new("id", "integer", false)],
                ..Table::default()
            },
        ],
        ..Catalog::default()
    };
    let mut layout = Diagram::default();
    layout
        .positions
        .insert("source".into(), Point { x: 100.0, y: 100.0 });
    layout
        .positions
        .insert("target".into(), Point { x: 750.0, y: 300.0 });
    for (id, points) in [
        (
            "a",
            vec![Point { x: 500.0, y: 148.0 }, Point { x: 500.0, y: 348.0 }],
        ),
        (
            "z",
            vec![
                Point { x: 600.0, y: 148.0 },
                Point { x: 600.0, y: 450.0 },
                Point { x: 700.0, y: 450.0 },
            ],
        ),
    ] {
        layout.routes.insert(
            id.into(),
            Route {
                points,
                source_anchor: Some(TableAnchor {
                    side: AnchorSide::Right,
                    offset: 48.0,
                }),
                target_anchor: Some(TableAnchor {
                    side: AnchorSide::Left,
                    offset: 48.0,
                }),
                ..Route::default()
            },
        );
    }
    let mut view = DiagramView::default();
    view.fit = false;
    view.zoom = 1.0;
    view.prepare_geometry(&catalog, &layout);
    let mut initialized = false;
    let mut harness = Harness::builder()
        .with_size([1400.0, 850.0])
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
                    None,
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
    let badge_texts = |h: &Harness<'_, State>| -> Vec<String> {
        h.output()
            .shapes
            .iter()
            .filter_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(t)
                    if ["0..N", "0..1"].contains(&t.galley.job.text.as_str()) =>
                {
                    Some(t.galley.job.text.clone())
                }
                _ => None,
            })
            .collect()
    };
    assert_eq!(badge_texts(&harness), vec!["0..N"]);
    let hover = harness.state().view.canvas.min + egui::vec2(600.0, 450.0);
    harness.hover_at(hover);
    harness.run();
    assert_eq!(badge_texts(&harness), vec!["0..1"]);
    harness.state_mut().view.selected_route = Some("a".into());
    harness.run();
    assert_eq!(badge_texts(&harness), vec!["0..N"]);
    harness.state_mut().view.selected_route = Some("z".into());
    harness.run();
    assert_eq!(badge_texts(&harness), vec!["0..1"]);
}
