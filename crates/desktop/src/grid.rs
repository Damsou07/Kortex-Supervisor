use crate::{columns, theme};
use egui::{RichText, Ui};
use egui_extras::{Column as UiColumn, TableBuilder};
use kortex_application::GridState;
use kortex_domain::*;
use std::{collections::BTreeMap, time::Instant};

pub struct DataView {
    pub table: Table,
    pub query: PageQuery,
    pub grid: GridState,
    pub pending: Option<u64>,
    pub applying: bool,
    pub last_refresh: Instant,
    pub page_stack: Vec<PageQuery>,
    pub error: Option<String>,
    pub selected_cell: Option<(usize, usize)>,
    pub edit_value: InputValue,
    pub refresh_requested: bool,
    pub schema_valid: bool,
    latest_table: Option<Table>,
}
impl DataView {
    pub fn new(table: Table) -> Self {
        Self {
            latest_table: Some(table.clone()),
            schema_valid: true,
            table,
            query: PageQuery::default(),
            grid: GridState::default(),
            pending: None,
            applying: false,
            last_refresh: Instant::now(),
            page_stack: vec![],
            error: None,
            selected_cell: None,
            edit_value: InputValue::Text(String::new()),
            refresh_requested: true,
        }
    }
    pub fn reconcile_table(&mut self, latest: Option<&Table>) {
        self.latest_table = latest.cloned();
        let Some(table) = latest else {
            self.schema_valid = false;
            self.pending = None;
            return;
        };
        let changed = table.id != self.table.id
            || table.columns != self.table.columns
            || table.identity_columns() != self.table.identity_columns()
            || table.is_view != self.table.is_view;
        if changed {
            // A page requested with the old column order must never reach this grid.
            self.pending = None;
            if self.grid.dirty() || self.applying {
                self.schema_valid = false;
                return;
            }
            self.grid.page = DataPage::default();
            self.query = PageQuery::default();
            self.page_stack.clear();
            self.selected_cell = None;
            self.refresh_requested = true;
        }
        self.schema_valid = true;
        self.table = table.clone();
    }
    pub fn discard_changes(&mut self) {
        self.grid.discard();
        self.selected_cell = None;
        let latest = self.latest_table.clone();
        self.reconcile_table(latest.as_ref());
        self.refresh_requested = true;
        self.error = None;
    }
}
#[derive(Default)]
pub struct GridAction {
    pub refresh: bool,
    pub apply: bool,
}
pub fn show(ui: &mut Ui, view: &mut DataView) -> GridAction {
    let mut action = GridAction::default();
    let dirty = view.grid.dirty();
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new(view.table.id.label()).strong().size(18.0));
        if view.pending.is_some() {
            ui.spinner();
        }
        if ui
            .add_enabled(view.pending.is_none(), egui::Button::new("↻ Actualiser"))
            .clicked()
        {
            action.refresh = true;
        }
        if ui
            .add_enabled(
                !view.table.is_view && !view.applying && view.schema_valid,
                egui::Button::new("+ Ligne"),
            )
            .clicked()
        {
            view.grid.inserted.push(BTreeMap::new());
        }
        if ui
            .add_enabled(
                dirty && !view.applying && view.pending.is_none() && view.schema_valid,
                egui::Button::new("Appliquer").fill(egui::Color32::from_rgb(110, 28, 35)),
            )
            .clicked()
        {
            action.apply = true;
        }
        if ui
            .add_enabled(
                dirty && !view.applying,
                egui::Button::new("Annuler les modifications"),
            )
            .clicked()
        {
            view.discard_changes();
            action.refresh = true;
        }
    });
    if let Some(error) = &view.error {
        ui.colored_label(theme::RED, error);
    }
    if !view.schema_valid {
        ui.colored_label(theme::RED, "La table a été modifiée ou supprimée. Le brouillon est conservé ; annulez-le pour recharger la structure actuelle.");
    }
    if view.grid.external_change {
        ui.colored_label(theme::YELLOW, "La base a changé. Vos modifications sont conservées ; les conflits seront contrôlés à l’application.");
    }
    if view.table.identity_columns().is_empty() {
        ui.label(
            RichText::new("Sans clé unique non nullable : mise à jour et suppression via SQL.")
                .color(theme::MUTED),
        );
    }
    egui::CollapsingHeader::new("Filtres et tris")
        .default_open(false)
        .show(ui, |ui| {
            ui.add_enabled_ui(!dirty && view.pending.is_none(), |ui| {
                let mut remove = None;
                for (i, filter) in view.query.filters.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        egui::ComboBox::new(("filter_col", i), "Colonne")
                            .selected_text(&filter.column)
                            .show_ui(ui, |ui| {
                                for col in &view.table.columns {
                                    ui.selectable_value(
                                        &mut filter.column,
                                        col.name.clone(),
                                        &col.name,
                                    );
                                }
                            });
                        egui::ComboBox::new(("filter_op", i), "Opérateur")
                            .selected_text(filter.op.label())
                            .show_ui(ui, |ui| {
                                for op in FilterOp::ALL {
                                    ui.selectable_value(&mut filter.op, op, op.label());
                                }
                            });
                        if !matches!(filter.op, FilterOp::IsNull | FilterOp::IsNotNull) {
                            ui.add(
                                egui::TextEdit::singleline(&mut filter.value)
                                    .hint_text("Valeur")
                                    .desired_width(130.0),
                            );
                        }
                        if ui.small_button("×").clicked() {
                            remove = Some(i);
                        }
                    });
                }
                if let Some(i) = remove {
                    view.query.filters.remove(i);
                }
                ui.horizontal(|ui| {
                    if ui.button("+ Filtre").clicked()
                        && let Some(col) = view.table.columns.first()
                    {
                        view.query.filters.push(Filter {
                            column: col.name.clone(),
                            ..Filter::default()
                        });
                    }
                    if ui.button("Filtrer").clicked() {
                        view.query.offset = 0;
                        view.query.after = None;
                        view.page_stack.clear();
                        action.refresh = true;
                    }
                    if ui.button("Réinitialiser").clicked() {
                        view.query = PageQuery::default();
                        view.page_stack.clear();
                        action.refresh = true;
                    }
                    ui.label("Clic sur un en-tête : tri · Maj+clic : tri supplémentaire");
                });
                if !view.query.sort.is_empty() {
                    ui.label(format!(
                        "Tri : {}",
                        view.query
                            .sort
                            .iter()
                            .map(|s| format!(
                                "{} {}",
                                s.column,
                                if s.descending { "↓" } else { "↑" }
                            ))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ));
                }
            });
        });
    if let Some((row, col)) = view.selected_cell {
        let column = &view.table.columns[col];
        ui.horizontal_wrapped(|ui| {
            ui.label(format!("{} · ligne {}", column.name, row + 1));
            input_ui(
                ui,
                &mut view.edit_value,
                !column.nullable,
                column.default.is_some(),
            );
            if ui
                .add_enabled(
                    !view.applying && view.schema_valid,
                    egui::Button::new("Valider la cellule"),
                )
                .clicked()
            {
                view.grid
                    .edits
                    .entry(row)
                    .or_default()
                    .insert(column.name.clone(), view.edit_value.clone());
                view.selected_cell = None;
            }
            if ui.button("Fermer").clicked() {
                view.selected_cell = None;
            }
        });
    }
    if !view.grid.inserted.is_empty() {
        egui::CollapsingHeader::new(format!("{} nouvelle(s) ligne(s)", view.grid.inserted.len()))
            .default_open(true)
            .show(ui, |ui| {
                egui::ScrollArea::both().max_height(160.0).show(ui, |ui| {
                    for (i, row) in view.grid.inserted.iter_mut().enumerate() {
                        ui.push_id(("insert", i), |ui| {
                            ui.horizontal(|ui| {
                                ui.label(format!("+ {}", i + 1));
                                for col in view.table.columns.iter().filter(|c| !c.generated) {
                                    ui.vertical(|ui| {
                                        ui.label(&col.name);
                                        input_ui(
                                            ui,
                                            row.entry(col.name.clone())
                                                .or_insert(InputValue::Default),
                                            !col.nullable,
                                            true,
                                        );
                                    });
                                }
                            });
                        });
                    }
                });
            });
    }
    ui.separator();
    let table = &view.table;
    columns::legend(ui);
    let rows = view.grid.page.rows.len();
    let can_edit = !table.identity_columns().is_empty() && !view.applying && view.schema_valid;
    let height = (ui.available_height() - 45.0).max(80.0);
    let mut sorted = None;
    egui::ScrollArea::horizontal().show(ui, |ui| {
        let mut builder = TableBuilder::new(ui)
            .striped(true)
            .resizable(true)
            .max_scroll_height(height)
            .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
            .column(UiColumn::initial(52.0).at_least(45.0));
        for _ in &table.columns {
            builder = builder.column(UiColumn::initial(200.0).at_least(100.0));
        }
        builder
            .header(48.0, |mut header| {
                header.col(|ui| {
                    ui.strong("Ligne");
                });
                for (i, col) in table.columns.iter().enumerate() {
                    header.col(|ui| {
                        if columns::header(ui, table, col, !dirty && view.pending.is_none())
                            .clicked()
                        {
                            sorted = Some((i, ui.input(|i| i.modifiers.shift)));
                        }
                    });
                }
            })
            .body(|body| {
                body.rows(29.0, rows, |mut row| {
                    let i = row.index();
                    let deleted = view.grid.deleted.contains(&i);
                    row.col(|ui| {
                        if ui
                            .add_enabled(
                                can_edit,
                                egui::Button::new(if deleted {
                                    "↶".into()
                                } else {
                                    format!("{} ×", view.query.offset + i + 1)
                                })
                                .frame(false),
                            )
                            .on_hover_text("Marquer / annuler la suppression")
                            .clicked()
                        {
                            if deleted {
                                view.grid.deleted.remove(&i);
                            } else {
                                view.grid.deleted.insert(i);
                            }
                        }
                    });
                    for (j, col) in table.columns.iter().enumerate() {
                        row.col(|ui| {
                            let original = &view.grid.page.rows[i].values[j];
                            let edited = view.grid.edits.get(&i).and_then(|row| row.get(&col.name));
                            let text = edited
                                .map(|v| match v {
                                    InputValue::Null => "NULL".into(),
                                    InputValue::Default => "DEFAULT".into(),
                                    InputValue::Text(s) => s.clone(),
                                })
                                .unwrap_or_else(|| original.display());
                            let clipped: String = text.chars().take(180).collect();
                            let mut rich = RichText::new(clipped);
                            if edited.is_some() {
                                rich = rich.color(theme::YELLOW);
                            } else if matches!(original, Value::Null) {
                                rich = rich.color(theme::MUTED).italics();
                            }
                            if deleted {
                                rich = rich.strikethrough().color(theme::RED);
                            }
                            let response = ui.add(
                                egui::Label::new(rich)
                                    .truncate()
                                    .sense(egui::Sense::click()),
                            );
                            if response.double_clicked() && can_edit && !col.generated && !deleted {
                                view.selected_cell = Some((i, j));
                                view.edit_value = edited
                                    .cloned()
                                    .unwrap_or_else(|| InputValue::from(original));
                            }
                            response.context_menu(|ui| {
                                if ui.button("Copier la valeur").clicked() {
                                    ui.ctx().copy_text(text);
                                    ui.close();
                                }
                            });
                        });
                    }
                });
            });
    });
    if let Some((i, additive)) = sorted {
        let name = table.columns[i].name.clone();
        let descending = view
            .query
            .sort
            .iter()
            .find(|s| s.column == name)
            .is_some_and(|s| !s.descending);
        if !additive {
            view.query.sort.clear();
        } else {
            view.query.sort.retain(|s| s.column != name);
        }
        view.query.sort.push(Sort {
            column: name,
            descending,
        });
        view.query.after = None;
        view.query.offset = 0;
        view.page_stack.clear();
        action.refresh = true;
    }
    ui.horizontal(|ui| {
        if ui
            .add_enabled(
                !dirty && !view.page_stack.is_empty() && view.pending.is_none(),
                egui::Button::new("← Précédent"),
            )
            .clicked()
            && let Some(q) = view.page_stack.pop()
        {
            view.query = q;
            action.refresh = true;
        }
        ui.label(format!(
            "{}–{} · {} lignes{}",
            view.query.offset + usize::from(rows > 0),
            view.query.offset + rows,
            rows,
            if view.grid.page.has_more {
                " · suite disponible"
            } else {
                ""
            }
        ));
        if ui
            .add_enabled(
                !dirty && view.grid.page.has_more && view.pending.is_none(),
                egui::Button::new("Suivant →"),
            )
            .clicked()
        {
            view.page_stack.push(view.query.clone());
            view.query.after = view
                .grid
                .page
                .rows
                .last()
                .and_then(|r| cursor_for(table, &view.query, r));
            view.query.offset += view.query.limit;
            action.refresh = true;
        }
        ui.label(
            RichText::new(format!(
                "Actualisé il y a {:.0} s",
                view.last_refresh.elapsed().as_secs_f32()
            ))
            .color(theme::MUTED),
        );
    });
    action
}
fn input_ui(ui: &mut Ui, value: &mut InputValue, required: bool, default_allowed: bool) {
    let mut kind = match value {
        InputValue::Text(_) => 0,
        InputValue::Null => 1,
        InputValue::Default => 2,
    };
    egui::ComboBox::new(ui.next_auto_id(), "Saisie")
        .selected_text(["Valeur", "NULL", "DEFAULT"][kind])
        .width(80.0)
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut kind, 0, "Valeur");
            ui.add_enabled_ui(!required, |ui| {
                ui.selectable_value(&mut kind, 1, "NULL");
            });
            ui.add_enabled_ui(default_allowed, |ui| {
                ui.selectable_value(&mut kind, 2, "DEFAULT");
            });
        });
    match kind {
        1 => *value = InputValue::Null,
        2 => *value = InputValue::Default,
        _ => {
            if !matches!(value, InputValue::Text(_)) {
                *value = InputValue::Text(String::new());
            }
            if let InputValue::Text(text) = value {
                ui.add(
                    egui::TextEdit::singleline(text)
                        .desired_width(160.0)
                        .hint_text("Valeur (vide autorisé)"),
                );
            }
        }
    }
}
