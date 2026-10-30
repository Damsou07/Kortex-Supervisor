//! Read-only presentation of catalog relations for the selected table.
use egui::{Label, RichText, Ui};
use kortex_domain::{Catalog, ForeignKey, Table, cardinalities};

fn column_name(columns: &[String], index: usize) -> &str {
    columns
        .get(index)
        .filter(|name| !name.is_empty())
        .map_or("non renseignée", String::as_str)
}

fn relation_name(fk: &ForeignKey) -> &str {
    if !fk.name.trim().is_empty() {
        &fk.name
    } else if !fk.id.trim().is_empty() {
        &fk.id
    } else {
        "Relation sans nom"
    }
}

fn show_relation_details(ui: &mut Ui, catalog: &Catalog, source: &Table, fk: &ForeignKey) {
    ui.add(Label::new(format!("{} → {}", source.id.label(), fk.target.label())).wrap());
    for index in 0..fk.columns.len().max(fk.target_columns.len()) {
        let mapping = format!(
            "{} → {}",
            column_name(&fk.columns, index),
            column_name(&fk.target_columns, index),
        );
        ui.add(Label::new(RichText::new(mapping).monospace()).wrap());
    }
    let (reference, inverse) = cardinalities(source, fk);
    ui.label(format!("Cardinalités : {} → {}", inverse.label(), reference.label()))
        .on_hover_text("Cardinalités affichées côté source puis côté cible, comme sur le MCD. Elles sont déduites des contraintes connues.");
    for (action, rule) in [("UPDATE", &fk.on_update), ("DELETE", &fk.on_delete)] {
        let rule = if rule.is_empty() {
            "Non renseigné"
        } else {
            rule
        };
        ui.add(Label::new(format!("ON {action} : {rule}")).wrap());
    }
    if !catalog.tables.iter().any(|table| table.id == fk.target) {
        ui.label("Cible absente du catalogue");
    }
}

pub(crate) fn show_relations(ui: &mut Ui, catalog: &Catalog, selected: &Table) {
    ui.label(crate::theme::heading("RELATIONS"));
    ui.push_id(("relations", &selected.id), |ui| {
        let mut count = 0;
        for source in &catalog.tables {
            for (index, fk) in source.foreign_keys.iter().enumerate() {
                let outgoing = source.id == selected.id;
                let incoming = fk.target == selected.id;
                let direction = match (outgoing, incoming) {
                    (true, true) => "Autoréférence",
                    (true, false) => "Sortante",
                    (false, true) => "Entrante",
                    (false, false) => continue,
                };
                count += 1;
                egui::CollapsingHeader::new(format!("{direction} · {}", relation_name(fk)))
                    .id_salt((&source.id, index))
                    .show(ui, |ui| show_relation_details(ui, catalog, source, fk));
            }
        }
        if count == 0 {
            ui.label("Aucune relation");
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui_kittest::{Harness, kittest::Queryable};
    use kortex_domain::{Column, ForeignKey, ObjectId};

    fn catalog() -> Catalog {
        let selected = Table {
            id: ObjectId::new("sales", "orders"),
            columns: vec![
                Column::new("tenant", "integer", false),
                Column::new("id", "integer", false),
            ],
            primary_key: vec!["tenant".into(), "id".into()],
            foreign_keys: vec![
                ForeignKey {
                    id: "customer_fk".into(),
                    name: "customer_fk".into(),
                    source: ObjectId::new("sales", "orders"),
                    columns: vec!["tenant".into(), "customer".into()],
                    target: ObjectId::new("crm", "customers"),
                    target_columns: vec!["tenant".into(), "id".into()],
                    on_update: "CASCADE".into(),
                    on_delete: "RESTRICT".into(),
                    ..ForeignKey::default()
                },
                ForeignKey {
                    id: "parent_fk".into(),
                    name: "parent_fk".into(),
                    source: ObjectId::new("sales", "orders"),
                    columns: vec!["parent_id".into()],
                    target: ObjectId::new("sales", "orders"),
                    target_columns: vec!["id".into()],
                    ..ForeignKey::default()
                },
            ],
            ..Table::default()
        };
        Catalog {
            tables: vec![
                selected,
                Table {
                    id: ObjectId::new("sales", "items"),
                    columns: vec![Column::new("order_id", "integer", false)],
                    foreign_keys: vec![ForeignKey {
                        id: "order_fk".into(),
                        name: "order_fk".into(),
                        source: ObjectId::new("sales", "items"),
                        columns: vec!["order_id".into()],
                        target: ObjectId::new("sales", "orders"),
                        target_columns: vec!["id".into()],
                        ..ForeignKey::default()
                    }],
                    ..Table::default()
                },
                Table {
                    id: ObjectId::new("archive", "orders"),
                    foreign_keys: vec![ForeignKey {
                        id: "unrelated_fk".into(),
                        name: "unrelated_fk".into(),
                        target: ObjectId::new("archive", "orders"),
                        ..ForeignKey::default()
                    }],
                    ..Table::default()
                },
            ],
            ..Catalog::default()
        }
    }

    #[test]
    fn relations_include_both_directions_self_once_and_missing_target_details() {
        let catalog = catalog();
        let original = catalog.clone();
        let mut initialized = false;
        let mut harness = Harness::builder()
            .with_size([340.0, 800.0])
            .build_ui(move |ui| {
                if !initialized {
                    crate::theme::apply(ui.ctx());
                    initialized = true;
                }
                show_relations(ui, &catalog, &catalog.tables[0]);
                assert_eq!(catalog, original);
            });
        harness.run();
        for label in [
            "RELATIONS",
            "Sortante · customer_fk",
            "Entrante · order_fk",
            "Autoréférence · parent_fk",
        ] {
            assert!(harness.query_by_label(label).is_some(), "missing {label}");
        }
        assert!(
            harness
                .query_by_label("Autoréférence · unrelated_fk")
                .is_none()
        );
        harness.get_by_label("Sortante · customer_fk").click();
        harness.run();
        for label in [
            "sales.orders → crm.customers",
            "tenant → tenant",
            "customer → id",
            "ON UPDATE : CASCADE",
            "ON DELETE : RESTRICT",
            "Cible absente du catalogue",
            "Cardinalités : 0..N → 0..1",
        ] {
            assert!(
                harness.query_by_label(label).is_some(),
                "missing detail {label}"
            );
        }
        assert_eq!(
            harness
                .get_all_by_label("Autoréférence · parent_fk")
                .count(),
            1
        );
        harness.snapshot("inspector_relations");
    }

    #[test]
    fn table_without_relations_has_an_explicit_empty_state() {
        let catalog = Catalog {
            tables: vec![Table::default()],
            ..Catalog::default()
        };
        let mut harness =
            Harness::builder().build_ui(move |ui| show_relations(ui, &catalog, &catalog.tables[0]));
        harness.run();
        assert!(harness.query_by_label("Aucune relation").is_some());
    }
}
