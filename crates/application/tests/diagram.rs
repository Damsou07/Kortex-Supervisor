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

#[test]
fn moving_members_reframes_all_affected_groups_in_the_same_history_step() {
    let catalog = Catalog {
        tables: ["a", "b", "c"]
            .into_iter()
            .map(|id| Table {
                stable_id: id.into(),
                id: ObjectId::new("main", id),
                ..Table::default()
            })
            .collect(),
        ..Catalog::default()
    };
    let mut layout = Diagram::default();
    layout.reconcile(&catalog.tables);
    let mut history = LayoutHistory::default();
    for members in [vec!["a".into(), "b".into()], vec!["b".into(), "c".into()]] {
        assert!(
            GroupDraft {
                name: "Groupe".into(),
                members,
                ..Default::default()
            }
            .save(&catalog, &mut layout, &mut history)
        );
    }
    let before = layout.clone();
    history.record(&layout);
    move_group(&catalog, &mut layout, 1, Point { x: 200.0, y: 30.0 });
    assert_ne!(layout.groups[0], before.groups[0]);
    assert_ne!(layout.groups[1], before.groups[1]);
    let moved = layout.clone();
    assert!(history.undo(&mut layout));
    assert_eq!(layout, before);
    assert!(history.redo(&mut layout));
    assert_eq!(layout, moved);
    move_tables(
        &catalog,
        &mut layout,
        &["c".into()],
        Point { x: 900.0, y: 0.0 },
    );
    let frame = &layout.groups[1];
    assert!(frame.origin.x + frame.size.x > layout.positions["c"].x);
}
