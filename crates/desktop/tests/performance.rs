use egui_kittest::Harness;
use kortex_domain::*;
use kortex_supervisor::{mcd, theme};
use std::time::Instant;

#[test]
#[ignore = "performance measurement; run in release mode"]
fn mcd_200_tables_500_relations() {
    let mut catalog = Catalog::default();
    for i in 0..200 {
        let mut table = Table {
            id: ObjectId::new("public", &format!("table_{i:03}")),
            stable_id: i.to_string(),
            columns: (0..8)
                .map(|c| Column::new(&format!("col_{c}"), "integer", c != 0))
                .collect(),
            primary_key: vec!["col_0".into()],
            ..Table::default()
        };
        for n in 0..if i < 100 { 3 } else { 2 } {
            table.foreign_keys.push(ForeignKey {
                id: format!("{i}_{n}"),
                source: table.id.clone(),
                target: ObjectId::new("public", &format!("table_{:03}", (i + n + 1) % 200)),
                columns: vec![format!("col_{}", n + 1)],
                target_columns: vec!["col_0".into()],
                ..ForeignKey::default()
            });
        }
        catalog.tables.push(table);
    }
    let mut diagram = Diagram::default();
    diagram.reconcile(&catalog.tables);
    let mut history = LayoutHistory::default();
    let mut initialized = false;
    let mut view = mcd::DiagramView::default();
    view.prepare_geometry(&catalog, &diagram);
    let mut harness = Harness::builder()
        .with_size([1400.0, 900.0])
        .build_ui_state(
            move |ui, view| {
                if !initialized {
                    theme::apply(ui.ctx());
                    initialized = true;
                }
                mcd::show(ui, &catalog, &mut diagram, &mut history, view, None);
            },
            view,
        );
    harness.run();
    let mut times = vec![];
    for _ in 0..120 {
        harness.state_mut().pan.x += 1.0;
        let start = Instant::now();
        harness.step();
        times.push(start.elapsed().as_secs_f64() * 1000.0);
    }
    times.sort_by(f64::total_cmp);
    let p95 = times[114];
    println!("MCD CPU frame p95: {p95:.2} ms (200 tables, 500 relations, 1400x900)");
    assert!(p95 < 33.0, "CPU frame budget exceeded: {p95:.2}ms");
}
