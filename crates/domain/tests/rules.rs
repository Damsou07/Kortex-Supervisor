use kortex_domain::*;

fn table() -> Table {
    Table {
        id: ObjectId::new("public", "orders"),
        stable_id: "42".into(),
        columns: vec![
            Column::new("id", "bigint", false),
            Column::new("customer_id", "bigint", true),
        ],
        primary_key: vec!["id".into()],
        ..Table::default()
    }
}

#[test]
fn nullable_fk_is_optional_and_unique_fk_limits_inverse() {
    let mut t = table();
    let fk = ForeignKey {
        columns: vec!["customer_id".into()],
        ..ForeignKey::default()
    };
    assert_eq!(
        cardinalities(&t, &fk),
        (Cardinality::OptionalOne, Cardinality::Many)
    );
    t.columns[1].nullable = false;
    t.unique_keys.push(vec!["customer_id".into()]);
    assert_eq!(
        cardinalities(&t, &fk),
        (Cardinality::One, Cardinality::OptionalOne)
    );
}

#[test]
fn match_full_composite_fk_with_required_part_is_required() {
    let t = table();
    let fk = ForeignKey {
        columns: vec!["id".into(), "customer_id".into()],
        match_full: true,
        ..ForeignKey::default()
    };
    assert_eq!(cardinalities(&t, &fk).0, Cardinality::One);
}

#[test]
fn nullable_unique_key_is_not_a_safe_row_identity() {
    let mut t = table();
    t.primary_key.clear();
    t.unique_keys.push(vec!["customer_id".into()]);
    assert!(t.identity_columns().is_empty());
    t.columns[1].nullable = false;
    assert_eq!(t.identity_columns(), vec!["customer_id"]);
}

#[test]
fn generated_filters_bind_values_and_quote_identifiers() {
    let mut t = table();
    t.columns[1].name = "odd\"name".into();
    let query = PageQuery {
        filters: vec![Filter {
            column: "odd\"name".into(),
            op: FilterOp::Equal,
            value: "x'; DROP TABLE orders; --".into(),
        }],
        ..PageQuery::default()
    };
    let built = build_page_query(&t, &query, Dialect::Postgres).unwrap();
    assert!(!built.sql.contains("DROP TABLE"));
    assert!(built.sql.contains("\"odd\"\"name\""));
    assert_eq!(
        built.params[0],
        Value::Text("x'; DROP TABLE orders; --".into())
    );
    let invalid = PageQuery {
        filters: vec![Filter {
            column: "missing".into(),
            op: FilterOp::IsNull,
            value: String::new(),
        }],
        ..PageQuery::default()
    };
    assert!(build_page_query(&t, &invalid, Dialect::Sqlite).is_err());
}

#[test]
fn refresh_preserves_layout_and_group_move_is_undoable() {
    let t = table();
    let mut d = Diagram::default();
    d.reconcile(std::slice::from_ref(&t));
    d.positions.insert("42".into(), Point { x: 10.0, y: 20.0 });
    d.reconcile(std::slice::from_ref(&t));
    assert_eq!(d.positions["42"].x, 10.0);
    let original = d.clone();
    let mut history = LayoutHistory::default();
    history.record(&d);
    d.move_tables(&["42".into()], Point { x: 5.0, y: -5.0 });
    assert_eq!(d.positions["42"].y, 15.0);
    assert!(history.undo(&mut d));
    assert_eq!(d, original);
    assert!(history.redo(&mut d));
    assert_eq!(d.positions["42"].x, 15.0);
}

#[test]
fn object_keys_do_not_collide_when_names_contain_dots() {
    assert_ne!(
        ObjectId::new("a.b", "c").key(),
        ObjectId::new("a", "b.c").key()
    );
}

#[test]
fn nonnullable_cursor_does_not_disable_index_range_scan() {
    let query = PageQuery {
        after: Some(vec![Value::Integer(900000)]),
        ..PageQuery::default()
    };
    let built = build_page_query(&table(), &query, Dialect::Sqlite).unwrap();
    assert!(!built.sql.contains("IS NULL"));
}
