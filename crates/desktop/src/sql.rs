use crate::theme;
use egui::{RichText, Ui};
use egui_extras::{Column, TableBuilder};
use kortex_domain::*;
use kortex_infrastructure::worker::Worker;

pub struct SqlView {
    pub text: String,
    pub worker: Option<Worker>,
    pub connected: bool,
    pub running: bool,
    pub request: u64,
    pub results: Vec<ResultSet>,
    pub elapsed_ms: u128,
    pub error: Option<String>,
    pub selected: Option<std::ops::Range<usize>>,
    pub limit: usize,
    pub history: Vec<String>,
    pub history_index: Option<usize>,
    pub omitted_results: usize,
    retained_rows: usize,
    receiving_omitted: bool,
}
#[derive(Default)]
pub struct ResultSet {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<Value>>,
    pub affected: u64,
    pub truncated: bool,
    pub complete: bool,
}
impl SqlView {
    pub fn new(limit: usize) -> Self {
        Self {
            text: "-- Ctrl+Entrée : exécuter la sélection ou le script\nSELECT 1 AS bienvenue;"
                .into(),
            worker: None,
            connected: false,
            running: false,
            request: 0,
            results: vec![],
            elapsed_ms: 0,
            error: None,
            selected: None,
            limit,
            history: vec![],
            history_index: None,
            omitted_results: 0,
            retained_rows: 0,
            receiving_omitted: false,
        }
    }
    pub fn clear_results(&mut self) {
        self.results.clear();
        self.retained_rows = 0;
        self.omitted_results = 0;
        self.receiving_omitted = false;
    }
    pub fn receive(&mut self, event: SqlEvent) {
        match event {
            SqlEvent::Columns(columns) => {
                self.receiving_omitted = self.results.len() >= 128;
                if self.receiving_omitted {
                    self.omitted_results += 1;
                } else {
                    self.results.push(ResultSet {
                        columns,
                        ..ResultSet::default()
                    });
                }
            }
            SqlEvent::Rows(rows) => {
                if !self.receiving_omitted
                    && let Some(result) = self.results.last_mut()
                {
                    let remaining = self.limit.saturating_sub(self.retained_rows);
                    result.truncated |= rows.len() > remaining;
                    let count = rows.len().min(remaining);
                    result.rows.extend(rows.into_iter().take(count));
                    self.retained_rows += count;
                }
            }
            SqlEvent::Complete {
                affected,
                truncated,
            } => {
                if self.receiving_omitted {
                    return;
                }
                if self.results.last().is_none_or(|r| r.complete) {
                    if self.results.len() >= 128 {
                        self.omitted_results += 1;
                        return;
                    }
                    self.results.push(ResultSet::default());
                }
                if let Some(result) = self.results.last_mut() {
                    result.affected = affected;
                    result.truncated |= truncated;
                    result.complete = true;
                }
            }
            SqlEvent::Error(e) => self.error = Some(e.to_string()),
            SqlEvent::Finished { elapsed_ms } => {
                self.elapsed_ms = elapsed_ms;
                self.running = false;
            }
        }
    }
    pub fn execution_text(&self) -> String {
        if let Some(range) = &self.selected {
            self.text
                .chars()
                .skip(range.start)
                .take(range.end.saturating_sub(range.start))
                .collect()
        } else {
            self.text.clone()
        }
    }
}
#[derive(Default)]
pub struct SqlAction {
    pub execute: bool,
    pub cancel: bool,
}
pub fn show(ui: &mut Ui, view: &mut SqlView, catalog: &Catalog) -> SqlAction {
    let mut action = SqlAction::default();
    ui.horizontal_wrapped(|ui| {
        if ui
            .add_enabled(
                view.connected && !view.running,
                egui::Button::new("▶ Exécuter").fill(egui::Color32::from_rgb(110, 28, 35)),
            )
            .clicked()
        {
            action.execute = true;
        }
        if ui
            .add_enabled(view.running, egui::Button::new("■ Arrêter"))
            .clicked()
        {
            action.cancel = true;
        }
        ui.label("Plafond d’affichage");
        ui.add(egui::DragValue::new(&mut view.limit).range(1..=100000));
        ui.label(RichText::new(format!("{} ms", view.elapsed_ms)).color(theme::MUTED));
        if view.running {
            ui.spinner();
        }
        ui.menu_button("Historique", |ui| {
            for text in view.history.iter().rev() {
                if ui
                    .button(text.chars().take(70).collect::<String>())
                    .clicked()
                {
                    view.text = text.clone();
                    ui.close();
                }
            }
        });
        ui.menu_button("Insérer un objet", |ui| {
            egui::ScrollArea::vertical()
                .max_height(280.0)
                .show(ui, |ui| {
                    for table in &catalog.tables {
                        ui.menu_button(table.id.label(), |ui| {
                            if ui.button("Nom de la table").clicked() {
                                view.text.push_str(&qualified(&table.id));
                                ui.close();
                            }
                            for c in &table.columns {
                                if ui.button(&c.name).clicked() {
                                    view.text.push_str(&quote(&c.name));
                                    ui.close();
                                }
                            }
                        });
                    }
                });
        });
    });
    ui.label(RichText::new("Session SQL dédiée · BEGIN / COMMIT / ROLLBACK disponibles · aucune relance automatique").color(theme::MUTED));
    if let Some(error) = &view.error {
        ui.colored_label(theme::RED, error);
    }
    if view.omitted_results > 0 {
        ui.colored_label(theme::YELLOW, format!("{} résultat(s) non affiché(s) : limite de 128 jeux de résultats. Le script continue son exécution.", view.omitted_results));
    }
    let mut layouter = |ui: &Ui, text: &dyn egui::TextBuffer, wrap_width: f32| {
        let mut job = highlight(text.as_str());
        job.wrap.max_width = wrap_width;
        ui.fonts_mut(|fonts| fonts.layout_job(job))
    };
    egui::Panel::top("sql_editor")
        .default_size(230.0)
        .min_size(100.0)
        .resizable(true)
        .show(ui, |ui| {
            egui::ScrollArea::both().show(ui, |ui| {
                ui.horizontal_top(|ui| {
                    ui.label(
                        RichText::new(
                            (1..=view.text.lines().count().max(1))
                                .map(|n| n.to_string())
                                .collect::<Vec<_>>()
                                .join("\n"),
                        )
                        .monospace()
                        .color(theme::MUTED),
                    );
                    let output = egui::TextEdit::multiline(&mut view.text)
                        .font(egui::TextStyle::Monospace)
                        .code_editor()
                        .desired_width(f32::INFINITY)
                        .desired_rows(12)
                        .layouter(&mut layouter)
                        .show(ui);
                    view.selected = output.cursor_range.and_then(|r| {
                        let range = r.as_sorted_char_range();
                        (!range.is_empty()).then_some(range.start.0..range.end.0)
                    });
                    if output.response.has_focus()
                        && ui.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::Enter))
                        && !view.running
                        && view.connected
                    {
                        action.execute = true;
                    }
                });
            });
        });
    egui::ScrollArea::vertical().show(ui, |ui| {
        if view.results.is_empty() {
            ui.label("Les résultats apparaîtront ici.");
        }
        for (i, result) in view.results.iter().enumerate() {
            ui.push_id(("result", i), |ui| {
                ui.horizontal(|ui| {
                    ui.strong(format!("Résultat {}", i + 1));
                    ui.label(format!(
                        "{} ligne(s) · {} affichée(s)",
                        result.affected,
                        result.rows.len()
                    ));
                    if result.truncated {
                        ui.colored_label(theme::YELLOW, "Affichage tronqué");
                    }
                });
                if !result.columns.is_empty() {
                    egui::ScrollArea::horizontal().show(ui, |ui| {
                        let mut builder = TableBuilder::new(ui)
                            .striped(true)
                            .resizable(true)
                            .max_scroll_height(300.0);
                        for _ in &result.columns {
                            builder = builder.column(Column::initial(160.0).at_least(70.0));
                        }
                        builder
                            .header(26.0, |mut h| {
                                for c in &result.columns {
                                    h.col(|ui| {
                                        ui.strong(c);
                                    });
                                }
                            })
                            .body(|body| {
                                body.rows(25.0, result.rows.len(), |mut row| {
                                    for value in &result.rows[row.index()] {
                                        row.col(|ui| {
                                            let text = value.display();
                                            ui.add(
                                                egui::Label::new(
                                                    text.chars().take(180).collect::<String>(),
                                                )
                                                .truncate(),
                                            )
                                            .context_menu(|ui| {
                                                if ui.button("Copier").clicked() {
                                                    ui.ctx().copy_text(text);
                                                    ui.close();
                                                }
                                            });
                                        });
                                    }
                                });
                            });
                    });
                }
                ui.separator();
            });
        }
    });
    action
}
fn highlight(sql: &str) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    let keywords = [
        "SELECT",
        "FROM",
        "WHERE",
        "JOIN",
        "LEFT",
        "RIGHT",
        "INNER",
        "OUTER",
        "ON",
        "AS",
        "AND",
        "OR",
        "NOT",
        "NULL",
        "INSERT",
        "INTO",
        "VALUES",
        "UPDATE",
        "SET",
        "DELETE",
        "CREATE",
        "ALTER",
        "DROP",
        "TABLE",
        "INDEX",
        "CONSTRAINT",
        "PRIMARY",
        "KEY",
        "FOREIGN",
        "REFERENCES",
        "UNIQUE",
        "BEGIN",
        "COMMIT",
        "ROLLBACK",
        "ORDER",
        "BY",
        "LIMIT",
        "GROUP",
        "HAVING",
        "RETURNING",
        "WITH",
    ];
    for line in sql.split_inclusive('\n') {
        if line.trim_start().starts_with("--") {
            job.append(
                line,
                0.0,
                egui::TextFormat {
                    font_id: egui::FontId::monospace(13.0),
                    color: theme::MUTED,
                    ..Default::default()
                },
            );
        } else {
            for part in line.split_inclusive(|c: char| !c.is_alphanumeric() && c != '_') {
                let word = part
                    .trim_matches(|c: char| !c.is_alphanumeric() && c != '_')
                    .to_uppercase();
                job.append(
                    part,
                    0.0,
                    egui::TextFormat {
                        font_id: egui::FontId::monospace(13.0),
                        color: if keywords.contains(&word.as_str()) {
                            theme::YELLOW
                        } else {
                            egui::Color32::from_gray(225)
                        },
                        ..Default::default()
                    },
                );
            }
        }
    }
    job
}
