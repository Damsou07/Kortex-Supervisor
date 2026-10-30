use super::{place_labels, relations};
use egui::{Rect, pos2, vec2};
use kortex_domain::{Catalog, Column, ForeignKey, ObjectId, Table};

#[test]
fn partial_view_collects_incoming_outgoing_and_internal_once() {
    let mut catalog = Catalog::default();
    for name in ["a", "b", "c"] {
        catalog.tables.push(Table {
            stable_id: name.into(),
            id: ObjectId::new("main", name),
            columns: vec![Column::new("id", "integer", false)],
            ..Table::default()
        });
    }
    for (source, target, id) in [
        (0, 1, "internal"),
        (0, 2, "out"),
        (2, 1, "in"),
        (2, 2, "hidden"),
        (1, 1, "self"),
    ] {
        let fk = ForeignKey {
            id: id.into(),
            source: catalog.tables[source].id.clone(),
            target: catalog.tables[target].id.clone(),
            ..ForeignKey::default()
        };
        catalog.tables[source].foreign_keys.push(fk);
    }
    let subset = vec!["a".into(), "b".into()];
    let collected = relations(&catalog, Some(&subset));
    let ids: Vec<_> = collected
        .iter()
        .map(|(_, _, fk, _, _)| fk.id.as_str())
        .collect();
    assert_eq!(ids, ["internal", "out", "self", "in"]);
}

#[test]
fn dense_labels_never_overlap_and_keep_their_order() {
    let labels = vec![Rect::from_min_size(pos2(20.0, 20.0), vec2(35.0, 12.0)); 20];
    let placed = place_labels(
        &labels,
        Rect::from_min_size(pos2(0.0, 0.0), vec2(800.0, 600.0)),
        &[],
    );
    assert_eq!(placed.len(), labels.len());
    for (i, a) in placed.iter().enumerate() {
        for b in &placed[i + 1..] {
            assert!(!a.intersects(*b));
        }
    }
    assert_eq!(placed[0], labels[0]);
}

#[test]
fn overlapping_labels_are_hidden_without_displacing_any_label() {
    let rect = Rect::from_min_size(pos2(20.0, 20.0), vec2(40.0, 20.0));
    let placed = place_labels(
        &[rect, rect],
        Rect::from_min_max(pos2(0.0, 0.0), pos2(800.0, 600.0)),
        &[],
    );
    assert_eq!(placed[0], rect);
    assert!(
        !placed[1].is_finite(),
        "a colliding label must be hidden, not moved"
    );
}

#[test]
fn labels_outside_the_canvas_are_hidden_instead_of_clamped() {
    let rect = Rect::from_min_size(pos2(-10.0, 20.0), vec2(40.0, 20.0));
    let placed = place_labels(
        &[rect],
        Rect::from_min_max(pos2(0.0, 0.0), pos2(800.0, 600.0)),
        &[],
    );
    assert!(!placed[0].is_finite());
}

#[test]
fn saturated_labels_are_reported_instead_of_overlapping() {
    let canvas = Rect::from_min_size(pos2(0.0, 0.0), vec2(90.0, 50.0));
    let labels = vec![Rect::from_min_size(pos2(4.0, 4.0), vec2(40.0, 12.0)); 20];
    let placed = place_labels(&labels, canvas, &[]);
    assert!(placed.iter().any(|rect| !rect.is_finite()));
    let visible: Vec<_> = placed.into_iter().filter(Rect::is_finite).collect();
    for (i, rect) in visible.iter().enumerate() {
        assert!(canvas.contains_rect(*rect));
        for other in &visible[i + 1..] {
            assert!(!rect.intersects(*other));
        }
    }
}

#[test]
fn full_width_labels_report_overflow_when_the_canvas_is_full() {
    let canvas = Rect::from_min_size(pos2(0.0, 0.0), vec2(90.0, 20.0));
    let labels = vec![Rect::from_min_size(pos2(2.0, 2.0), vec2(86.0, 12.0)); 2];
    let placed = place_labels(&labels, canvas, &[]);
    assert!(canvas.contains_rect(placed[0]));
    assert!(!placed[1].is_finite());
}

#[test]
fn badges_stay_outside_their_table_and_preserve_fixed_dimensions() {
    use kortex_domain::AnchorSide;
    let table = Rect::from_min_max(pos2(100.0, 100.0), pos2(420.0, 300.0));
    for (side, endpoint) in [
        (AnchorSide::Left, pos2(100.0, 200.0)),
        (AnchorSide::Right, pos2(420.0, 200.0)),
        (AnchorSide::Top, pos2(250.0, 100.0)),
        (AnchorSide::Bottom, pos2(250.0, 300.0)),
    ] {
        for source in [true, false] {
            let badge = super::badge_rect(endpoint, side, source);
            assert_eq!(badge.size(), vec2(40.0, 20.0));
            assert!(!badge.intersects(table));
        }
    }
}

#[test]
fn badges_covering_tables_are_hidden_and_reappear_when_clear() {
    let badge = Rect::from_min_size(pos2(20.0, 20.0), vec2(40.0, 20.0));
    let canvas = Rect::from_min_max(pos2(0.0, 0.0), pos2(800.0, 600.0));
    assert!(!place_labels(&[badge], canvas, &[badge])[0].is_finite());
    assert_eq!(place_labels(&[badge], canvas, &[])[0], badge);
}

#[test]
fn generated_elbows_do_not_change_manual_waypoint_insertion_order() {
    let points = [
        pos2(0.0, 0.0),
        pos2(24.0, 0.0),
        pos2(24.0, 30.0),
        pos2(50.0, 30.0),
        pos2(80.0, 30.0),
        pos2(100.0, 30.0),
        pos2(100.0, 50.0),
    ];
    let manual = [points[3], points[4]];
    assert_eq!(super::waypoint_insertion_index(&points, &manual, 1), 0);
    assert_eq!(super::waypoint_insertion_index(&points, &manual, 3), 1);
    assert_eq!(super::waypoint_insertion_index(&points, &manual, 5), 2);
}

#[test]
fn remote_names_never_overlap_the_fixed_badge_on_any_side() {
    use kortex_domain::AnchorSide;
    for side in [
        AnchorSide::Left,
        AnchorSide::Right,
        AnchorSide::Top,
        AnchorSide::Bottom,
    ] {
        for source in [true, false] {
            let endpoint = pos2(400.0, 400.0);
            let badge = super::badge_rect(endpoint, side, source);
            let name = super::endpoint_label_rect(endpoint, side, !source, vec2(180.0, 16.0));
            assert!(!badge.intersects(name));
        }
    }
}
