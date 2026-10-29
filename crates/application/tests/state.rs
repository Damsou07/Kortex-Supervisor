use kortex_application::*;
use kortex_domain::*;

#[test]
fn older_results_cannot_replace_newer_requests() {
    let mut gate = RequestGate::default();
    let a = gate.begin(4);
    let b = gate.begin(4);
    assert!(!gate.accepts(4, a));
    assert!(gate.accepts(4, b));
    assert!(!gate.accepts(9, b));
}
#[test]
fn refresh_does_not_destroy_pending_edits() {
    let row = DataRow {
        values: vec![Value::Text("before".into())],
        version: None,
    };
    let mut grid = GridState::default();
    grid.receive(DataPage {
        rows: vec![row.clone()],
        has_more: false,
    });
    grid.edits
        .entry(0)
        .or_default()
        .insert("name".into(), InputValue::Text("mine".into()));
    grid.receive(DataPage {
        rows: vec![DataRow {
            values: vec![Value::Text("theirs".into())],
            version: None,
        }],
        has_more: false,
    });
    assert_eq!(grid.page.rows[0], row);
    assert!(grid.external_change);
    assert_eq!(grid.changes().changes.len(), 1);
}
