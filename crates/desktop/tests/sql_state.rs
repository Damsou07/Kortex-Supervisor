use kortex_domain::*;
use kortex_supervisor::sql::SqlView;

#[test]
fn sql_display_budget_is_shared_by_all_result_sets() {
    let mut view = SqlView::new(3);
    for _ in 0..200 {
        view.receive(SqlEvent::Columns(vec!["value".into()]));
        view.receive(SqlEvent::Rows(vec![vec![Value::Integer(1)]; 2]));
        view.receive(SqlEvent::Complete {
            affected: 2,
            truncated: false,
        });
    }
    assert_eq!(view.results.iter().map(|r| r.rows.len()).sum::<usize>(), 3);
    assert_eq!(view.results.len(), 128);
    assert!(view.results[1].truncated);
}
