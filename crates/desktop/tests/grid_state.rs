use kortex_domain::*;
use kortex_supervisor::grid::DataView;

#[test]
fn schema_refresh_blocks_writes_and_keeps_draft_until_discard() {
    let table = Table {
        stable_id: "1".into(),
        id: ObjectId::new("main", "items"),
        columns: vec![Column::new("name", "TEXT", true)],
        ..Table::default()
    };
    let mut view = DataView::new(table.clone());
    view.grid.inserted.push(Default::default());
    let mut renamed = table.clone();
    renamed.columns[0].name = "label".into();
    view.reconcile_table(Some(&renamed));
    assert!(!view.schema_valid);
    assert!(view.grid.dirty());
    assert_eq!(view.table, table);
    view.discard_changes();
    assert!(view.schema_valid);
    assert_eq!(view.table, renamed);
    assert!(!view.grid.dirty());
    assert!(view.refresh_requested);
}

#[test]
fn dropped_table_cannot_receive_new_edits_or_be_applied() {
    let mut view = DataView::new(Table::default());
    view.reconcile_table(None);
    assert!(!view.schema_valid);
    view.discard_changes();
    assert!(!view.schema_valid);
}
