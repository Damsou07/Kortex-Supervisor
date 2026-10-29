use kortex_application::diagram::*;
use kortex_domain::*;

#[test]
fn group_draft_commits_once_and_frame_deletion_never_deletes_tables() {
    let catalog = Catalog {
        tables: vec![Table {
            stable_id: "a".into(),
            columns: vec![Column::new("id", "integer", false)],
            ..Table::default()
        }],
        ..Catalog::default()
    };
    let mut layout = Diagram::default();
    layout.reconcile(&catalog.tables);
    let before = layout.clone();
    let mut history = LayoutHistory::default();
    let mut draft = GroupDraft {
        name: "  Domaine  ".into(),
        members: vec!["a".into(), "missing".into(), "a".into()],
        color: [80, 120, 160, 40],
        ..GroupDraft::default()
    };
    assert!(draft.save(&catalog, &mut layout, &mut history));
    assert_eq!(layout.groups[0].members, vec!["a"]);
    assert_eq!(layout.groups[0].name, "Domaine");
    assert!(history.undo(&mut layout));
    assert_eq!(layout, before);
    assert!(history.redo(&mut layout));
    draft.id = Some(1);
    draft.name = "Renommé".into();
    assert!(draft.save(&catalog, &mut layout, &mut history));
    remove_group(1, &mut layout, &mut history);
    assert!(layout.groups.is_empty());
    assert_eq!(layout.positions, before.positions);
    assert!(history.undo(&mut layout));
    assert_eq!(layout.groups[0].name, "Renommé");
}
#[test]
fn invalid_group_is_a_noop_and_default_layout_is_one_undo_step() {
    let catalog = Catalog {
        tables: vec![Table {
            stable_id: "a".into(),
            id: ObjectId::new("main", "a"),
            ..Table::default()
        }],
        ..Catalog::default()
    };
    let mut layout = Diagram::default();
    layout.reconcile(&catalog.tables);
    let mut history = LayoutHistory::default();
    assert!(!GroupDraft::default().save(&catalog, &mut layout, &mut history));
    assert!(!history.undo(&mut layout));
    layout.positions.get_mut("a").unwrap().x += 500.0;
    let before = layout.clone();
    let result = arrange_diagram(&LayoutInput::new(&catalog, &layout));
    apply_arrangement(&catalog, &mut layout, &mut history, &result, None);
    assert!(history.undo(&mut layout));
    assert_eq!(layout, before);
}
