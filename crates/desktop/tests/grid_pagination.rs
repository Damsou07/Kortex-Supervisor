use egui_kittest::{Harness, kittest::Queryable};
use kortex_domain::*;
use kortex_supervisor::{
    grid::{self, DataView},
    theme,
};

struct State {
    view: DataView,
    refresh: bool,
}

fn harness(size: [f32; 2]) -> Harness<'static, State> {
    let table = Table {
        id: ObjectId::new("main", "pagination"),
        columns: (0..12)
            .map(|i| Column::new(&format!("col_{i}"), "INTEGER", false))
            .collect(),
        primary_key: vec!["col_0".into()],
        ..Default::default()
    };
    let mut view = DataView::new(table);
    view.grid.page.rows = (0..200)
        .map(|i| DataRow {
            values: (0..12).map(|j| Value::Integer(i * 12 + j)).collect(),
            version: None,
        })
        .collect();
    view.grid.page.has_more = true;
    let mut initialized = false;
    Harness::builder().with_size(size).build_ui_state(
        move |ui, state: &mut State| {
            if !initialized {
                theme::apply(ui.ctx());
                initialized = true;
            }
            state.refresh |= grid::show(ui, &mut state.view).refresh;
        },
        State {
            view,
            refresh: false,
        },
    )
}

fn assert_pagination_inside(harness: &Harness<'_, State>, size: [f32; 2]) {
    let bounds = egui::Rect::from_min_size(egui::Pos2::ZERO, size.into());
    for label in ["← Précédent", "Suivant →"] {
        let rect = harness.get_by_label(label).rect();
        assert!(
            bounds.contains_rect(rect),
            "{label} clipped at {size:?}: {rect:?}"
        );
    }
}

#[test]
fn full_page_pagination_stays_visible_after_resize_and_opening_controls() {
    let mut h = harness([1000.0, 650.0]);
    h.run();
    assert_pagination_inside(&h, [1000.0, 650.0]);
    for size in [[650.0, 350.0], [340.0, 260.0]] {
        h.set_size(size.into());
        h.run();
        assert_pagination_inside(&h, size);
    }
    h.state_mut().view.query.filters = (0..8)
        .map(|_| Filter {
            column: "col_0".into(),
            ..Default::default()
        })
        .collect();
    h.get_by_label("Filtres et tris").click();
    h.run();
    h.state_mut().view.selected_cell = Some((0, 1));
    h.run();
    assert_pagination_inside(&h, [340.0, 260.0]);
    h.snapshot("grid_pagination_narrow");
}

#[test]
fn page_navigation_preserves_queries_and_guards_pending_and_dirty_states() {
    let mut h = harness([1000.0, 650.0]);
    h.run();
    let first = h.state().view.query.clone();
    h.get_by_label("← Précédent").click();
    h.run();
    assert_eq!(h.state().view.query.offset, 0);
    h.get_by_label("Suivant →").click();
    h.run();
    assert_eq!(h.state().view.query.offset, 200);
    assert!(h.state().refresh);
    assert_eq!(h.state().view.page_stack, vec![first.clone()]);
    assert!(h.state().view.query.after.is_some());
    h.get_by_label("← Précédent").click();
    h.run();
    assert_eq!(h.state().view.query, first);
    h.state_mut().refresh = false;
    h.state_mut().view.pending = Some(42);
    h.run_steps(2);
    h.get_by_label("Suivant →").click();
    h.run_steps(2);
    assert_eq!(h.state().view.query.offset, 0);
    assert!(!h.state().refresh);
    h.state_mut().view.pending = None;
    h.state_mut()
        .view
        .grid
        .edits
        .entry(0)
        .or_default()
        .insert("col_1".into(), InputValue::Text("3".into()));
    h.run();
    h.get_by_label("Suivant →").click();
    h.run();
    assert_eq!(h.state().view.query.offset, 0);
    h.state_mut().view.grid.discard();
    h.state_mut().view.grid.page.has_more = false;
    h.run();
    h.get_by_label("Suivant →").click();
    h.run();
    assert_eq!(h.state().view.query.offset, 0);
}
