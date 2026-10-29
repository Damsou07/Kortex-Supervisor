use kortex_domain::*;

fn catalog() -> Catalog {
    Catalog {
        tables: (0..8)
            .map(|i| Table {
                id: ObjectId::new("main", &format!("t{i}")),
                stable_id: i.to_string(),
                columns: vec![Column::new("id", "integer", false)],
                foreign_keys: if i == 0 {
                    vec![]
                } else {
                    vec![ForeignKey {
                        id: format!("fk{i}"),
                        source: ObjectId::new("main", &format!("t{i}")),
                        target: ObjectId::new("main", "t0"),
                        columns: vec!["id".into()],
                        target_columns: vec!["id".into()],
                        ..ForeignKey::default()
                    }]
                },
                ..Table::default()
            })
            .collect(),
        ..Catalog::default()
    }
}

#[test]
fn compact_layout_is_deterministic_and_ports_are_separate() {
    let input = LayoutInput::new(&catalog(), &Diagram::default());
    let result = arrange_diagram(&input);
    assert_eq!(result, arrange_diagram(&input));
    let rects: Vec<_> = result.rects.values().collect();
    for (i, a) in rects.iter().enumerate() {
        for b in &rects[i + 1..] {
            assert!(!a.intersects(b));
        }
    }
    let ports: Vec<_> = result
        .edges
        .values()
        .map(|e| *e.points.last().unwrap())
        .collect();
    for (i, a) in ports.iter().enumerate() {
        for b in &ports[i + 1..] {
            assert!((a.x - b.x).hypot(a.y - b.y) >= 16.0);
        }
    }
    for (id, edge) in &result.edges {
        assert!(
            edge.points
                .windows(2)
                .all(|s| s[0].x == s[1].x || s[0].y == s[1].y),
            "{id}"
        );
        for rect in result.rects.values() {
            assert!(
                edge.points
                    .windows(2)
                    .all(|s| !rect.segment_crosses_interior(s[0], s[1])),
                "{id} crosses a table"
            );
        }
    }
}

#[test]
fn fixed_tables_and_saved_manual_routes_are_preserved() {
    let mut d = Diagram::default();
    d.positions.insert("0".into(), Point { x: 72.0, y: 81.0 });
    let mut input = LayoutInput::new(&catalog(), &d);
    input.fixed.insert("0".into());
    let result = arrange_diagram(&input);
    assert_eq!(result.rects["0"].origin, d.positions["0"]);
    assert_eq!(d.positions.len(), 1);
}

#[test]
fn selection_rect_only_returns_intersecting_tables() {
    let input = LayoutInput::new(&catalog(), &Diagram::default());
    let result = arrange_diagram(&input);
    let rect = result.rects["0"];
    assert_eq!(result.tables_in(rect), vec!["0".to_owned()]);
}

#[test]
fn cycles_self_links_parallel_edges_and_overlapping_groups_keep_all_relations() {
    let mut c = catalog();
    let (first, second, third) = (
        c.tables[0].id.clone(),
        c.tables[1].id.clone(),
        c.tables[2].id.clone(),
    );
    c.tables[0].foreign_keys.push(ForeignKey {
        id: "cycle".into(),
        source: first,
        target: second,
        ..ForeignKey::default()
    });
    c.tables[2].foreign_keys.push(ForeignKey {
        id: "self".into(),
        source: third.clone(),
        target: third,
        ..ForeignKey::default()
    });
    let mut parallel = c.tables[3].foreign_keys[0].clone();
    parallel.id = "parallel".into();
    c.tables[3].foreign_keys.push(parallel);
    let mut input = LayoutInput::new(&c, &Diagram::default());
    input.groups = vec![vec!["0".into(), "1".into()], vec!["1".into(), "2".into()]];
    let result = arrange_diagram(&input);
    assert_eq!(result.edges.len(), input.edges.len());
    for (id, edge) in &result.edges {
        assert!(edge.points.len() > 1, "{id}");
        for r in result.rects.values() {
            assert!(
                edge.points
                    .windows(2)
                    .all(|s| !r.segment_crosses_interior(s[0], s[1])),
                "{id}"
            );
        }
    }
    let bounds = |ids: &[&str]| {
        let rects: Vec<_> = ids.iter().map(|id| result.rects[*id]).collect();
        rects.iter().map(|r| r.right()).fold(0.0, f32::max)
            - rects
                .iter()
                .map(|r| r.origin.x)
                .fold(f32::INFINITY, f32::min)
    };
    assert!(bounds(&["0", "1", "2"]) < 1200.0);
}

#[test]
fn refresh_places_new_tables_without_overlapping_or_moving_existing_ones() {
    let mut c = catalog();
    let new = c.tables.pop().unwrap();
    let mut d = Diagram::default();
    d.reconcile(&c.tables);
    let old = d.positions.clone();
    c.tables.push(new);
    d.reconcile(&c.tables);
    for (id, p) in old {
        assert_eq!(d.positions[&id], p);
    }
    let input = LayoutInput::new(&c, &d);
    let new = input.nodes.iter().find(|n| n.id == "7").unwrap();
    for n in &input.nodes {
        if n.id != "7" {
            assert!(!new.rect.intersects(&n.rect));
        }
    }
}

#[test]
fn a_single_source_anchor_stays_next_to_its_foreign_key_column() {
    let mut c = catalog();
    c.tables[1]
        .columns
        .push(Column::new("parent_id", "integer", false));
    c.tables[1].foreign_keys[0].columns = vec!["parent_id".into()];
    let result = arrange_diagram(&LayoutInput::new(&c, &Diagram::default()));
    assert_eq!(
        result.edges["fk1"].points[0].y - result.rects["1"].origin.y,
        72.0
    );
}

#[test]
fn subset_arrangement_preserves_manual_routes_between_fixed_tables() {
    let c = catalog();
    let mut d = Diagram::default();
    d.reconcile(&c.tables);
    d.routes.insert(
        "fk1".into(),
        Route {
            points: vec![Point { x: 900.0, y: 30.0 }, Point { x: 910.0, y: 30.0 }],
            source_y: Some(22.0),
            target_y: Some(38.0),
            ..Route::default()
        },
    );
    let mut input = LayoutInput::new(&c, &d);
    input.fixed = std::collections::BTreeSet::from(["0".into(), "1".into()]);
    let expected = route_diagram(&input);
    let arranged = arrange_diagram(&input);
    assert_eq!(arranged.edges["fk1"].points, expected.edges["fk1"].points);
}

#[test]
fn crossing_paths_receive_a_bridge_without_merging_relations() {
    let mut c = Catalog {
        tables: ["a", "b", "c", "d"]
            .iter()
            .map(|name| Table {
                stable_id: name.to_string(),
                id: ObjectId::new("main", name),
                columns: vec![Column::new("id", "integer", false)],
                ..Table::default()
            })
            .collect(),
        ..Catalog::default()
    };
    for (source, target, id) in [(0, 1, "ab"), (2, 3, "cd")] {
        let fk = ForeignKey {
            id: id.into(),
            source: c.tables[source].id.clone(),
            target: c.tables[target].id.clone(),
            columns: vec!["id".into()],
            target_columns: vec!["id".into()],
            ..ForeignKey::default()
        };
        c.tables[source].foreign_keys.push(fk);
    }
    let mut d = Diagram::default();
    for (id, x, y) in [
        ("a", 0.0, 100.0),
        ("b", 800.0, 100.0),
        ("c", 400.0, -200.0),
        ("d", 400.0, 400.0),
    ] {
        d.positions.insert(id.into(), Point { x, y });
    }
    d.routes.insert(
        "ab".into(),
        Route {
            points: vec![Point { x: 560.0, y: 148.0 }],
            source_y: Some(48.0),
            target_y: Some(48.0),
            ..Route::default()
        },
    );
    d.routes.insert(
        "cd".into(),
        Route {
            points: vec![
                Point {
                    x: 760.0,
                    y: -152.0,
                },
                Point { x: 760.0, y: 250.0 },
                Point { x: 360.0, y: 250.0 },
                Point { x: 360.0, y: 448.0 },
            ],
            source_y: Some(48.0),
            target_y: Some(48.0),
            ..Route::default()
        },
    );
    let result = route_diagram(&LayoutInput::new(&c, &d));
    assert_eq!(result.edges.len(), 2);
    assert!(
        result.edges["cd"]
            .bridges
            .contains(&Point { x: 760.0, y: 148.0 })
    );
}
