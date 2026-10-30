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
fn spacious_profile_increases_spacing_without_moving_fixed_tables() {
    let mut diagram = Diagram::default();
    diagram
        .positions
        .insert("0".into(), Point { x: 72.0, y: 81.0 });
    let mut input = LayoutInput::new(&catalog(), &diagram);
    input.fixed.insert("0".into());
    let compact = arrange_diagram_with_profile(&input, LayoutProfile::Compact);
    let spacious = arrange_diagram_with_profile(&input, LayoutProfile::Spacious);
    assert_eq!(compact.rects["0"].origin, diagram.positions["0"]);
    assert_eq!(spacious.rects["0"].origin, diagram.positions["0"]);
    assert_ne!(compact.rects, spacious.rects);
    assert_eq!(
        spacious,
        arrange_diagram_with_profile(&input, LayoutProfile::Spacious)
    );
    let rects: Vec<_> = spacious.rects.values().collect();
    for (i, a) in rects.iter().enumerate() {
        for b in &rects[i + 1..] {
            assert!(!a.intersects(b));
        }
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

fn anchor_rect(x: f32, y: f32) -> NodeRect {
    NodeRect {
        origin: Point { x, y },
        size: Point { x: 320.0, y: 160.0 },
    }
}

fn four_side_input() -> LayoutInput {
    LayoutInput {
        nodes: vec![
            LayoutNode {
                id: "a".into(),
                rect: anchor_rect(0.0, 0.0),
            },
            LayoutNode {
                id: "b".into(),
                rect: anchor_rect(640.0, 320.0),
            },
            LayoutNode {
                id: "obstacle".into(),
                rect: anchor_rect(320.0, 160.0),
            },
        ],
        edges: vec![LayoutEdge {
            id: "ab".into(),
            source: "a".into(),
            target: "b".into(),
            source_y: 48.0,
            target_y: 48.0,
        }],
        ..LayoutInput::default()
    }
}

fn assert_orthogonal_clear(points: &[Point], obstacles: &[NodeRect]) {
    assert!(points.len() >= 2);
    for pair in points.windows(2) {
        assert!(
            pair[0].x == pair[1].x || pair[0].y == pair[1].y,
            "diagonal: {pair:?}"
        );
        for rect in obstacles {
            assert!(
                !rect.segment_crosses_interior(pair[0], pair[1]),
                "{pair:?} crosses {rect:?}"
            );
        }
    }
}

#[test]
fn four_side_projection_resolves_local_offsets_and_stable_corner_ties() {
    let rect = anchor_rect(100.0, 200.0);
    for (pointer, expected) in [
        (
            Point { x: 90.0, y: 250.0 },
            TableAnchor {
                side: AnchorSide::Left,
                offset: 50.0,
            },
        ),
        (
            Point { x: 430.0, y: 250.0 },
            TableAnchor {
                side: AnchorSide::Right,
                offset: 50.0,
            },
        ),
        (
            Point { x: 250.0, y: 190.0 },
            TableAnchor {
                side: AnchorSide::Top,
                offset: 150.0,
            },
        ),
        (
            Point { x: 250.0, y: 370.0 },
            TableAnchor {
                side: AnchorSide::Bottom,
                offset: 150.0,
            },
        ),
    ] {
        assert_eq!(project_table_anchor(rect, pointer, None), expected);
        let point = resolve_table_anchor(rect, expected);
        let moved = resolve_table_anchor(anchor_rect(120.0, 230.0), expected);
        assert_eq!(
            moved,
            Point {
                x: point.x + 20.0,
                y: point.y + 30.0
            }
        );
    }
    let corner = Point { x: 100.0, y: 200.0 };
    assert_eq!(
        project_table_anchor(rect, corner, Some(AnchorSide::Top)).side,
        AnchorSide::Top
    );
    assert_eq!(
        project_table_anchor(rect, corner, Some(AnchorSide::Left)).side,
        AnchorSide::Left
    );
    let anchor = TableAnchor {
        side: AnchorSide::Left,
        offset: 200.0,
    };
    assert_eq!(resolve_table_anchor(rect, anchor).y, rect.bottom());
    assert_eq!(
        anchor.offset, 200.0,
        "resolving a smaller table must not rewrite placement"
    );
}

#[test]
fn manual_anchors_route_outward_on_all_four_sides_without_crossing_tables() {
    for source_side in [
        AnchorSide::Left,
        AnchorSide::Right,
        AnchorSide::Top,
        AnchorSide::Bottom,
    ] {
        for target_side in [
            AnchorSide::Left,
            AnchorSide::Right,
            AnchorSide::Top,
            AnchorSide::Bottom,
        ] {
            let mut input = four_side_input();
            let source = TableAnchor {
                side: source_side,
                offset: 80.0,
            };
            let target = TableAnchor {
                side: target_side,
                offset: 80.0,
            };
            input.manual.insert(
                "ab".into(),
                Route {
                    source_anchor: Some(source),
                    target_anchor: Some(target),
                    ..Route::default()
                },
            );
            let output = route_diagram(&input);
            let points = &output.edges["ab"].points;
            assert_eq!(points[0], resolve_table_anchor(output.rects["a"], source));
            assert_eq!(
                *points.last().unwrap(),
                resolve_table_anchor(output.rects["b"], target)
            );
            let departure = Point {
                x: points[1].x - points[0].x,
                y: points[1].y - points[0].y,
            };
            let last = points.len() - 1;
            let arrival = Point {
                x: points[last - 1].x - points[last].x,
                y: points[last - 1].y - points[last].y,
            };
            for (delta, normal) in [
                (departure, source_side.normal()),
                (arrival, target_side.normal()),
            ] {
                assert!(delta.x * normal.x + delta.y * normal.y > 0.0);
            }
            assert_orthogonal_clear(
                points,
                &input.nodes.iter().map(|node| node.rect).collect::<Vec<_>>(),
            );
        }
    }
}

#[test]
fn endpoint_reconnection_preserves_waypoints_and_avoids_source_and_target_interiors() {
    let input = four_side_input();
    let source_rect = input.nodes[0].rect;
    let target_rect = input.nodes[1].rect;
    let original = [
        Point { x: 320.0, y: 48.0 },
        Point { x: 400.0, y: 48.0 },
        Point { x: 1000.0, y: 48.0 },
        Point {
            x: 1000.0,
            y: 368.0,
        },
        Point { x: 960.0, y: 368.0 },
    ];
    let source = TableAnchor {
        side: AnchorSide::Top,
        offset: 80.0,
    };
    let target = TableAnchor {
        side: AnchorSide::Bottom,
        offset: 100.0,
    };
    let obstacles: Vec<_> = input.nodes.iter().map(|node| node.rect).collect();
    let connected = reconnect_route_endpoints(
        &original,
        (source_rect, source),
        (target_rect, target),
        &obstacles,
    );
    assert_eq!(connected[0], resolve_table_anchor(source_rect, source));
    assert_eq!(
        *connected.last().unwrap(),
        resolve_table_anchor(target_rect, target)
    );
    for point in &original[1..original.len() - 1] {
        assert!(connected.contains(point), "lost waypoint {point:?}");
    }
    assert_orthogonal_clear(&connected, &obstacles);
}

#[test]
fn coincident_manual_self_anchors_keep_a_visible_exterior_loop() {
    for side in [
        AnchorSide::Left,
        AnchorSide::Right,
        AnchorSide::Top,
        AnchorSide::Bottom,
    ] {
        let rect = anchor_rect(100.0, 100.0);
        let anchor = TableAnchor { side, offset: 80.0 };
        let mut input = four_side_input();
        input.nodes = vec![LayoutNode {
            id: "a".into(),
            rect,
        }];
        input.edges[0].target = "a".into();
        input.manual.insert(
            "ab".into(),
            Route {
                source_anchor: Some(anchor),
                target_anchor: Some(anchor),
                ..Route::default()
            },
        );
        let result = route_diagram(&input);
        let points = &result.edges["ab"].points;
        assert_eq!(points.first(), points.last());
        assert!(points.len() >= 5, "collapsed self loop: {points:?}");
        assert_orthogonal_clear(points, &[rect]);
    }
}

#[test]
fn tight_obstacle_gap_keeps_live_and_final_connectors_outside_tables() {
    let mut input = four_side_input();
    input.nodes[2].rect = anchor_rect(336.0, 0.0);
    let source = TableAnchor {
        side: AnchorSide::Right,
        offset: 80.0,
    };
    let target = TableAnchor {
        side: AnchorSide::Left,
        offset: 80.0,
    };
    input.manual.insert(
        "ab".into(),
        Route {
            source_anchor: Some(source),
            target_anchor: Some(target),
            ..Route::default()
        },
    );
    let obstacles: Vec<_> = input.nodes.iter().map(|node| node.rect).collect();
    let live = reconnect_route_endpoints(
        &[],
        (input.nodes[0].rect, source),
        (input.nodes[1].rect, target),
        &obstacles,
    );
    assert_orthogonal_clear(&live, &obstacles);
    assert_orthogonal_clear(&route_diagram(&input).edges["ab"].points, &obstacles);
}

#[test]
fn coincident_self_loop_remains_visible_in_a_narrow_obstacle_gap() {
    let mut input = four_side_input();
    input.nodes[2].rect = anchor_rect(336.0, 0.0);
    input.edges[0].target = "a".into();
    let anchor = TableAnchor {
        side: AnchorSide::Right,
        offset: 80.0,
    };
    input.manual.insert(
        "ab".into(),
        Route {
            source_anchor: Some(anchor),
            target_anchor: Some(anchor),
            ..Route::default()
        },
    );
    let obstacles: Vec<_> = input.nodes.iter().map(|node| node.rect).collect();
    let points = route_diagram(&input).edges["ab"].points.clone();
    assert!(
        points.len() >= 5,
        "loop collapsed to an out-and-back: {points:?}"
    );
    assert!(points.iter().any(|point| point.y != points[0].y));
    assert_orthogonal_clear(&points, &obstacles);
}

#[test]
fn live_reconnection_finds_a_narrow_corridor_in_the_visibility_fallback() {
    let rect = |x, y, width, height| NodeRect {
        origin: Point { x, y },
        size: Point {
            x: width,
            y: height,
        },
    };
    let source = rect(20.0, 320.0, 10.0, 60.0);
    let target = rect(380.0, 20.0, 10.0, 60.0);
    // The left chamber can only escape between x=200 and x=208, then turn
    // above the right wall. No one- or two-elbow connector can leave it.
    let obstacles = [
        source,
        target,
        rect(100.0, 0.0, 100.0, 300.0),
        rect(208.0, 100.0, 112.0, 300.0),
        rect(0.0, 200.0, 100.0, 100.0),
        rect(0.0, 380.0, 208.0, 100.0),
        rect(-100.0, 200.0, 100.0, 280.0),
    ];
    let points = reconnect_route_endpoints(
        &[],
        (
            source,
            TableAnchor {
                side: AnchorSide::Right,
                offset: 30.0,
            },
        ),
        (
            target,
            TableAnchor {
                side: AnchorSide::Left,
                offset: 30.0,
            },
        ),
        &obstacles,
    );
    assert_orthogonal_clear(&points, &obstacles);
    assert!(
        points
            .iter()
            .any(|point| point.x >= 200.0 && point.x <= 208.0)
    );
}

#[test]
fn self_links_with_different_sides_and_corner_anchors_stay_outside_the_table() {
    for source_side in [
        AnchorSide::Left,
        AnchorSide::Right,
        AnchorSide::Top,
        AnchorSide::Bottom,
    ] {
        for target_side in [
            AnchorSide::Left,
            AnchorSide::Right,
            AnchorSide::Top,
            AnchorSide::Bottom,
        ] {
            let rect = anchor_rect(100.0, 100.0);
            let mut input = four_side_input();
            input.nodes = vec![LayoutNode {
                id: "a".into(),
                rect,
            }];
            input.edges[0].target = "a".into();
            input.manual.insert(
                "ab".into(),
                Route {
                    source_anchor: Some(TableAnchor {
                        side: source_side,
                        offset: 0.0,
                    }),
                    target_anchor: Some(TableAnchor {
                        side: target_side,
                        offset: 0.0,
                    }),
                    ..Route::default()
                },
            );
            let result = route_diagram(&input);
            let points = &result.edges["ab"].points;
            assert!(points.windows(2).any(|pair| pair[0] != pair[1]));
            assert_orthogonal_clear(points, &[rect]);
        }
    }
}

#[test]
fn manual_four_side_route_keeps_user_waypoints_and_legacy_routes_keep_their_shape() {
    let mut input = four_side_input();
    let waypoints = vec![
        Point { x: 400.0, y: -80.0 },
        Point {
            x: 1000.0,
            y: -80.0,
        },
    ];
    input.manual.insert(
        "ab".into(),
        Route {
            points: waypoints.clone(),
            source_y: Some(48.0),
            target_y: Some(48.0),
            ..Route::default()
        },
    );
    let legacy = route_diagram(&input).edges["ab"].points.clone();
    assert_eq!(&legacy[1..legacy.len() - 1], &waypoints);
    input.manual.get_mut("ab").unwrap().source_anchor = Some(TableAnchor {
        side: AnchorSide::Top,
        offset: 80.0,
    });
    input.manual.get_mut("ab").unwrap().target_anchor = Some(TableAnchor {
        side: AnchorSide::Right,
        offset: 80.0,
    });
    let points = route_diagram(&input).edges["ab"].points.clone();
    for waypoint in waypoints {
        assert!(points.contains(&waypoint));
    }
    assert_orthogonal_clear(
        &points,
        &input.nodes.iter().map(|node| node.rect).collect::<Vec<_>>(),
    );
}
