use egui_kittest::{Harness, kittest::Queryable};
use kortex_domain::*;
use kortex_supervisor::mcd::{self, DiagramView, InteractionMode};

struct State {
    catalog: Catalog,
    layout: Diagram,
    history: LayoutHistory,
    view: DiagramView,
}
#[test]
fn rectangle_selects_tables_without_selecting_a_relation() {
    let mut catalog = Catalog::default();
    for name in ["a", "b"] {
        catalog.tables.push(Table {
            stable_id: name.to_string(),
            id: ObjectId::new("main", name),
            columns: vec![Column::new("id", "integer", false)],
            ..Table::default()
        });
    }
    let (source, target) = (catalog.tables[0].id.clone(), catalog.tables[1].id.clone());
    catalog.tables[0].foreign_keys.push(ForeignKey {
        id: "fk".into(),
        source,
        target,
        columns: vec!["id".into()],
        target_columns: vec!["id".into()],
        ..ForeignKey::default()
    });
    let mut layout = Diagram::default();
    layout
        .positions
        .insert("a".into(), Point { x: 50.0, y: 50.0 });
    layout
        .positions
        .insert("b".into(), Point { x: 450.0, y: 50.0 });
    let mut view = DiagramView::default();
    view.fit = false;
    view.zoom = 1.0;
    view.mode = InteractionMode::Select;
    view.prepare_geometry(&catalog, &layout);
    let mut initialized = false;
    let mut harness = Harness::builder()
        .with_size([1100.0, 700.0])
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
    let canvas = harness.state().view.canvas;
    harness.drag_at(canvas.min + egui::vec2(30.0, 30.0));
    harness.step();
    harness.hover_at(canvas.min + egui::vec2(800.0, 200.0));
    harness.step();
    harness.drop_at(canvas.min + egui::vec2(800.0, 200.0));
    harness.run();
    assert_eq!(harness.state().view.selected.len(), 2);
    assert!(harness.state().view.selected_route.is_none());
    harness.state_mut().view.selected = std::collections::BTreeSet::from(["a".into()]);
    let (start, end) = (
        canvas.min + egui::vec2(430.0, 30.0),
        canvas.min + egui::vec2(790.0, 200.0),
    );
    harness.hover_at(start);
    harness.step();
    harness.event(egui::Event::ModifiersChanged(egui::Modifiers::SHIFT));
    harness.event(egui::Event::PointerButton {
        pos: start,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: egui::Modifiers::SHIFT,
    });
    harness.step();
    harness.event(egui::Event::PointerMoved(end));
    harness.step();
    harness.event(egui::Event::PointerButton {
        pos: end,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: egui::Modifiers::SHIFT,
    });
    harness.run();
    assert_eq!(harness.state().view.selected.len(), 2);
    harness.event(egui::Event::ModifiersChanged(egui::Modifiers::NONE));
    harness.step();
    harness.get_by_label("Groupes").click();
    harness.run();
    harness.get_by_label("+ Nouveau groupe").click();
    harness.run();
    assert_eq!(
        harness
            .state()
            .view
            .group_draft
            .as_ref()
            .unwrap()
            .members
            .len(),
        2
    );
    harness.snapshot("groups");
    harness.get_by_label("Annuler").click();
    harness.run();
    assert!(harness.state().layout.groups.is_empty());
    harness.get_by_label("+ Nouveau groupe").click();
    harness.run();
    harness.get_by_label("Valider le groupe").click();
    harness.run();
    assert_eq!(harness.state().layout.groups[0].members.len(), 2);
}

fn has_text(shape: &egui::epaint::Shape, text: &str) -> bool {
    match shape {
        egui::epaint::Shape::Text(t) => t.galley.job.text == text,
        egui::epaint::Shape::Vec(children) => children.iter().any(|s| has_text(s, text)),
        _ => false,
    }
}

#[test]
fn column_text_is_emitted_at_every_zoom_and_loupe_exposes_full_type() {
    let catalog = Catalog {
        tables: vec![Table {
            stable_id: "t".into(),
            id: ObjectId::new("main", "lecture"),
            columns: vec![Column::new("α_nom", "character varying(255)", true)],
            ..Table::default()
        }],
        ..Catalog::default()
    };
    let mut layout = Diagram::default();
    layout.positions.insert("t".into(), Point::default());
    let mut view = DiagramView::default();
    view.fit = false;
    view.zoom = 0.12;
    view.prepare_geometry(&catalog, &layout);
    let mut initialized = false;
    let mut harness = Harness::builder()
        .with_size([1100.0, 700.0])
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
    for zoom in [0.12, 0.25, 0.27, 1.0, 2.5] {
        harness.state_mut().view.zoom = zoom;
        harness.run();
        assert!(
            harness
                .output()
                .shapes
                .iter()
                .any(|s| has_text(&s.shape, "α_nom")),
            "missing column at {zoom}"
        );
    }
    harness.state_mut().view.zoom = 0.25;
    harness.run();
    let p = harness.state().view.canvas.min + egui::vec2(30.0, 16.0);
    harness.hover_at(p);
    // Egui's tooltip timer advances with test frames rather than wall-clock sleeps.
    for _ in 0..45 {
        harness.step();
    }
    assert!(harness.query_by_label("character varying(255)").is_some());
    harness.get_by_role(egui::Role::TextInput).click();
    harness.run();
    harness.key_combination_modifiers(egui::Modifiers::CTRL, &[egui::Key::A]);
    harness.run();
    assert!(harness.state().view.selected.is_empty());
    harness.get_by_label("Zone du MCD").click();
    harness.run();
    harness.key_combination_modifiers(egui::Modifiers::CTRL, &[egui::Key::A]);
    harness.run();
    assert_eq!(harness.state().view.selected.len(), 1);
}
