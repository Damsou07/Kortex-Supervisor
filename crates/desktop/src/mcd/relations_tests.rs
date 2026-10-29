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
fn saturated_labels_are_reported_instead_of_overlapping() {
    let canvas = Rect::from_min_size(pos2(0.0, 0.0), vec2(90.0, 50.0));
    let labels = vec![Rect::from_min_size(pos2(0.0, 0.0), vec2(40.0, 12.0)); 20];
    let placed = place_labels(&labels, canvas);
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
    let labels = vec![Rect::from_min_size(pos2(0.0, 0.0), vec2(90.0, 12.0)); 2];
    let placed = place_labels(&labels, canvas);
    assert!(canvas.contains_rect(placed[0]));
    assert!(!placed[1].is_finite());
}

#[test]
fn overflow_description_identifies_direction_columns_and_cardinality() {
    let source = Table {
        id: ObjectId::new("sales", "orders"),
        ..Table::default()
    };
    let target = Table {
        id: ObjectId::new("sales", "customers"),
        ..Table::default()
    };
    let fk = ForeignKey {
        name: "fk_customer".into(),
        columns: vec!["customer_id".into()],
        target_columns: vec!["id".into()],
        ..ForeignKey::default()
    };
    let text = super::overflow_description(&source, &target, &fk, "0..N", false, true);
    for part in [
        "fk_customer",
        "orders",
        "customer_id",
        "customers",
        "id",
        "0..N",
        "entrant",
    ] {
        assert!(text.contains(part), "missing {part} in {text}");
    }
}
