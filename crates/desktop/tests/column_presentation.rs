use kortex_domain::*;
use kortex_supervisor::columns::{ColumnRole, TypeFamily, roles, short_type, type_family};

#[test]
fn display_aliases_keep_parameters_arrays_and_unknown_types() {
    assert_eq!(short_type("character varying(255)[]"), "varchar(255)[]");
    assert_eq!(short_type("timestamp(3) with time zone"), "timestamptz(3)");
    assert_eq!(short_type("timestamp without time zone"), "timestamp");
    assert_eq!(short_type("public.État_métier"), "public.État_métier");
    assert_eq!(
        short_type("character varying_custom"),
        "character varying_custom"
    );
    assert_eq!(
        short_type("timestamp_custom with time zone"),
        "timestamp_custom with time zone"
    );
    assert_eq!(type_family("NUMERIC(18,2)"), TypeFamily::Number);
    assert_eq!(type_family("jsonb[]"), TypeFamily::Json);
    assert_eq!(type_family("public.text_domain"), TypeFamily::Other);
}

#[test]
fn simultaneous_key_roles_are_distinct_and_composite_unique_is_preserved() {
    let mut table = Table {
        columns: vec![Column::new("id", "integer", false)],
        primary_key: vec!["id".into()],
        unique_keys: vec![vec!["id".into(), "tenant".into()]],
        ..Table::default()
    };
    table.foreign_keys.push(ForeignKey {
        columns: vec!["id".into()],
        ..ForeignKey::default()
    });
    assert_eq!(
        roles(&table, &table.columns[0]),
        vec![ColumnRole::Primary, ColumnRole::Foreign, ColumnRole::Unique]
    );
}
