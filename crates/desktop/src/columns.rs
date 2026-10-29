//! Shared, display-only column metadata. SQL identifiers and declared types stay intact.
use crate::theme;
use egui::{Align2, Color32, FontId, Painter, Rect, Response, Sense, Stroke, Ui, Vec2};
use kortex_domain::{Column, Table};
use kortex_infrastructure::storage::{Appearance, LegendPalette, LegendStyle};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TypeFamily {
    Number,
    Text,
    Boolean,
    Date,
    Uuid,
    Json,
    Binary,
    Other,
}
impl TypeFamily {
    pub fn color(self) -> Color32 {
        match self {
            Self::Number => Color32::from_rgb(138, 180, 248),
            Self::Text => Color32::from_rgb(129, 201, 149),
            Self::Boolean => Color32::from_rgb(196, 161, 255),
            Self::Date => Color32::from_rgb(246, 178, 107),
            Self::Uuid => Color32::from_rgb(99, 215, 208),
            Self::Json => Color32::from_rgb(230, 211, 111),
            Self::Binary => Color32::from_rgb(193, 169, 147),
            Self::Other => theme::MUTED,
        }
    }
}

pub fn short_type(declared: &str) -> String {
    fn parameters(s: &str) -> bool {
        s.is_empty()
            || s.strip_prefix('(')
                .and_then(|s| s.strip_suffix(')'))
                .is_some_and(|s| {
                    !s.is_empty()
                        && s.chars()
                            .all(|c| c.is_ascii_digit() || c == ',' || c == ' ')
                })
    }
    let lower = declared.to_lowercase();
    let mut value = lower.as_str();
    let mut suffix = String::new();
    while let Some(base) = value.strip_suffix("[]") {
        suffix.push_str("[]");
        value = base;
    }
    let alias = if let Some(rest) = value.strip_prefix("character varying") {
        parameters(rest).then(|| format!("varchar{rest}"))
    } else if let Some(rest) = value.strip_prefix("timestamp") {
        rest.strip_suffix(" with time zone")
            .filter(|p| parameters(p))
            .map(|p| format!("timestamptz{p}"))
            .or_else(|| {
                rest.strip_suffix(" without time zone")
                    .filter(|p| parameters(p))
                    .map(|p| format!("timestamp{p}"))
            })
    } else if let Some(rest) = value.strip_prefix("time") {
        rest.strip_suffix(" with time zone")
            .filter(|p| parameters(p))
            .map(|p| format!("timetz{p}"))
            .or_else(|| {
                rest.strip_suffix(" without time zone")
                    .filter(|p| parameters(p))
                    .map(|p| format!("time{p}"))
            })
    } else if value == "double precision" {
        Some("float8".into())
    } else {
        None
    };
    alias
        .map(|a| format!("{a}{suffix}"))
        .unwrap_or_else(|| declared.to_owned())
}

pub fn type_family(declared: &str) -> TypeFamily {
    let short = short_type(declared).to_lowercase();
    let base = short.split(['(', '[']).next().unwrap_or("").trim();
    match base {
        "smallint" | "integer" | "bigint" | "int" | "int2" | "int4" | "int8" | "real" | "float"
        | "float4" | "float8" | "double" | "numeric" | "decimal" | "serial" | "bigserial"
        | "money" => TypeFamily::Number,
        "text" | "varchar" | "char" | "character" | "clob" | "citext" => TypeFamily::Text,
        "boolean" | "bool" => TypeFamily::Boolean,
        "date" | "time" | "timetz" | "timestamp" | "timestamptz" | "datetime" | "interval" => {
            TypeFamily::Date
        }
        "uuid" => TypeFamily::Uuid,
        "json" | "jsonb" => TypeFamily::Json,
        "blob" | "bytea" | "binary" | "varbinary" => TypeFamily::Binary,
        _ => TypeFamily::Other,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColumnRole {
    Primary,
    Foreign,
    Unique,
}
impl ColumnRole {
    pub fn label(self) -> &'static str {
        match self {
            Self::Primary => "PK",
            Self::Foreign => "FK",
            Self::Unique => "UQ",
        }
    }
    pub fn color(self) -> Color32 {
        match self {
            Self::Primary => theme::YELLOW,
            Self::Foreign => Color32::from_rgb(112, 200, 255),
            Self::Unique => Color32::from_rgb(217, 160, 255),
        }
    }
}
pub fn roles(table: &Table, column: &Column) -> Vec<ColumnRole> {
    let mut roles = Vec::new();
    if table.primary_key.contains(&column.name) {
        roles.push(ColumnRole::Primary);
    }
    if table
        .foreign_keys
        .iter()
        .any(|f| f.columns.contains(&column.name))
    {
        roles.push(ColumnRole::Foreign);
    }
    if table.unique_keys.iter().any(|k| k.contains(&column.name)) {
        roles.push(ColumnRole::Unique);
    }
    roles
}
pub fn tooltip(table: &Table, column: &Column) -> String {
    let keys = roles(table, column)
        .iter()
        .map(|r| match r {
            ColumnRole::Primary => "PK · clé primaire",
            ColumnRole::Foreign => "FK · clé étrangère",
            ColumnRole::Unique => "UQ · membre d’une clé unique (éventuellement composite)",
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "{}\n{}\n{}{}\n{}\n{}",
        column.name,
        column.data_type,
        if column.nullable {
            "NULL autorisé"
        } else {
            "NOT NULL"
        },
        if column.generated {
            " · générée"
        } else {
            ""
        },
        keys,
        column.comment
    )
}

/// Measure actual glyphs; character slicing never splits a UTF-8 code point.
pub fn ellipsize(painter: &Painter, text: &str, font: &FontId, width: f32) -> String {
    let measure = |s: &str| {
        painter
            .layout_no_wrap(s.to_owned(), font.clone(), Color32::WHITE)
            .size()
            .x
    };
    if measure(text) <= width {
        return text.to_owned();
    }
    if measure("…") > width {
        return String::new();
    }
    let chars: Vec<_> = text.chars().collect();
    let (mut low, mut high) = (0, chars.len());
    while low < high {
        let mid = (low + high).div_ceil(2);
        let candidate = format!("{}…", chars[..mid].iter().collect::<String>());
        if measure(&candidate) <= width {
            low = mid;
        } else {
            high = mid - 1;
        }
    }
    format!("{}…", chars[..low].iter().collect::<String>())
}

pub fn paint_badge(painter: &Painter, origin: egui::Pos2, role: ColumnRole, scale: f32) {
    let p = |x, y| origin + egui::vec2(x, y) * scale;
    let stroke = Stroke::new((1.1 * scale).max(0.3), role.color());
    match role {
        ColumnRole::Primary => {
            painter.add(egui::Shape::closed_line(
                vec![
                    p(3.5, 1.0),
                    p(6.5, 3.5),
                    p(6.5, 6.5),
                    p(3.5, 9.0),
                    p(0.5, 6.5),
                    p(0.5, 3.5),
                ],
                stroke,
            ));
            painter.circle_filled(p(3.5, 5.0), 0.9 * scale, role.color());
            painter.line_segment([p(6.5, 5.0), p(12.0, 5.0)], stroke);
            painter.line_segment([p(9.0, 5.0), p(9.0, 8.0)], stroke);
            painter.line_segment([p(11.0, 5.0), p(11.0, 7.0)], stroke);
        }
        ColumnRole::Foreign => {
            painter.circle_stroke(p(3.0, 8.0), 2.3 * scale, stroke);
            painter.circle_stroke(p(9.0, 3.0), 2.3 * scale, stroke);
            painter.line_segment([p(4.5, 6.5), p(7.5, 4.5)], stroke);
        }
        ColumnRole::Unique => {
            painter.add(egui::Shape::closed_line(
                vec![p(6.0, 1.0), p(11.0, 6.0), p(6.0, 11.0), p(1.0, 6.0)],
                stroke,
            ));
        }
    }
    painter.text(
        p(15.0, 0.0),
        Align2::LEFT_TOP,
        role.label(),
        FontId::monospace(10.0 * scale),
        role.color(),
    );
}

pub fn paint_row(painter: &Painter, rect: Rect, table: &Table, column: &Column, scale: f32) {
    paint_row_with_palette(
        painter,
        rect,
        table,
        column,
        scale,
        Color32::from_gray(228),
        theme::MUTED,
    );
}

pub fn paint_row_with_palette(
    painter: &Painter,
    rect: Rect,
    table: &Table,
    column: &Column,
    scale: f32,
    text: Color32,
    muted: Color32,
) {
    let painter = painter.with_clip_rect(rect.intersect(painter.clip_rect()));
    let role_list = roles(table, column);
    let mut x = rect.left() + 8.0 * scale;
    for role in role_list {
        paint_badge(
            &painter,
            egui::pos2(x, rect.top() + 5.0 * scale),
            role,
            scale,
        );
        x += 34.0 * scale;
    }
    let status_width = if column.nullable || column.generated {
        12.0 * scale
    } else {
        0.0
    };
    let content_right = rect.right() - status_width;
    let split = (content_right - 100.0 * scale).max(x);
    let font = FontId::monospace(11.0 * scale);
    let name = ellipsize(
        &painter,
        &column.name,
        &font,
        (split - x - 6.0 * scale).max(0.0),
    );
    painter.text(
        egui::pos2(x, rect.top() + 5.0 * scale),
        Align2::LEFT_TOP,
        name,
        font,
        text,
    );
    let font = FontId::monospace(10.0 * scale);
    let data_type = ellipsize(
        &painter,
        &short_type(&column.data_type),
        &font,
        (content_right - split - 8.0 * scale).max(0.0),
    );
    painter.text(
        egui::pos2(content_right - 8.0 * scale, rect.top() + 5.0 * scale),
        Align2::RIGHT_TOP,
        data_type,
        font,
        type_family(&column.data_type).color(),
    );
    if column.nullable {
        painter.circle_stroke(
            egui::pos2(rect.right() - 7.0 * scale, rect.center().y),
            2.4 * scale,
            Stroke::new(scale.max(0.2), muted),
        );
    }
    if column.generated {
        painter.text(
            egui::pos2(rect.right() - 7.0 * scale, rect.top() + 2.0 * scale),
            Align2::CENTER_TOP,
            "ƒ",
            FontId::proportional(10.0 * scale),
            muted,
        );
    }
}

pub fn header(ui: &mut Ui, table: &Table, column: &Column, enabled: bool) -> Response {
    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(ui.available_width().max(40.0), 42.0),
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, enabled, &column.name));
    let rect = rect.shrink2(egui::vec2(4.0, 0.0));
    let painter = ui.painter().with_clip_rect(rect.intersect(ui.clip_rect()));
    let split = rect.left() + rect.width() * 0.55;
    let font = FontId::proportional(12.0);
    painter.text(
        rect.min,
        Align2::LEFT_TOP,
        ellipsize(
            &painter,
            &column.name,
            &font,
            (split - rect.left() - 4.0).max(0.0),
        ),
        font,
        Color32::WHITE,
    );
    let font = FontId::monospace(10.0);
    painter.text(
        rect.right_top(),
        Align2::RIGHT_TOP,
        ellipsize(
            &painter,
            &short_type(&column.data_type),
            &font,
            (rect.right() - split).max(0.0),
        ),
        font,
        type_family(&column.data_type).color(),
    );
    let mut x = rect.left();
    for role in roles(table, column) {
        paint_badge(&painter, egui::pos2(x, rect.top() + 23.0), role, 1.0);
        x += 35.0;
    }
    if column.nullable || column.generated {
        painter.text(
            egui::pos2(x, rect.top() + 23.0),
            Align2::LEFT_TOP,
            if column.nullable && column.generated {
                "NULL · générée"
            } else if column.generated {
                "générée"
            } else {
                "NULL"
            },
            FontId::monospace(9.0),
            theme::MUTED,
        );
    }
    response.on_hover_text(tooltip(table, column))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct LegendColors {
    surface: Color32,
    card: Color32,
    border: Color32,
    text: Color32,
    muted: Color32,
}

fn legend_colors(appearance: Appearance) -> LegendColors {
    match appearance.legend_palette {
        LegendPalette::FollowDiagram => {
            let palette = theme::palette_for(appearance.diagram_theme);
            LegendColors {
                surface: palette.canvas,
                card: palette.card,
                border: palette.border,
                text: palette.text,
                muted: palette.muted,
            }
        }
        LegendPalette::Neutral => LegendColors {
            surface: Color32::from_rgb(20, 22, 27),
            card: Color32::from_rgb(31, 34, 41),
            border: Color32::from_rgb(82, 88, 101),
            text: Color32::from_rgb(234, 237, 242),
            muted: Color32::from_rgb(170, 177, 190),
        },
        LegendPalette::HighContrast => LegendColors {
            surface: Color32::BLACK,
            card: Color32::from_gray(12),
            border: Color32::WHITE,
            text: Color32::WHITE,
            muted: Color32::from_gray(215),
        },
    }
}

const LEGEND_FAMILIES: [(&str, TypeFamily, &str); 8] = [
    ("Nombres", TypeFamily::Number, "integer · numeric(18,2)"),
    ("Textes", TypeFamily::Text, "text · varchar(255)"),
    ("Booléens", TypeFamily::Boolean, "boolean · bool"),
    ("Dates et heures", TypeFamily::Date, "date · timestamptz"),
    ("UUID", TypeFamily::Uuid, "uuid"),
    ("JSON", TypeFamily::Json, "json · jsonb"),
    ("Binaires", TypeFamily::Binary, "bytea · blob"),
    (
        "Autres types",
        TypeFamily::Other,
        "domaines et types personnalisés",
    ),
];

pub fn legend(ui: &mut Ui) {
    ui.menu_button("Légende", |ui| {
        // Establish the popup width before nested frames and scrolling can
        // reduce the available width to the remaining space in a chip row.
        ui.set_width(360.0);
        let mut appearance = theme::appearance(ui.ctx());
        ui.horizontal(|ui| {
            for (style, label) in [
                (LegendStyle::Compact, "Compacte"),
                (LegendStyle::Detailed, "Détaillée"),
                (LegendStyle::Cards, "Cartes"),
            ] {
                ui.selectable_value(&mut appearance.legend_style, style, label);
            }
        });
        egui::ComboBox::from_label("Palette")
            .selected_text(match appearance.legend_palette {
                LegendPalette::FollowDiagram => "Suivre le MCD",
                LegendPalette::Neutral => "Neutre",
                LegendPalette::HighContrast => "Contraste élevé",
            })
            .show_ui(ui, |ui| {
                for (palette, label) in [
                    (LegendPalette::FollowDiagram, "Suivre le MCD"),
                    (LegendPalette::Neutral, "Neutre"),
                    (LegendPalette::HighContrast, "Contraste élevé"),
                ] {
                    ui.selectable_value(&mut appearance.legend_palette, palette, label);
                }
            });
        theme::set_appearance(ui.ctx(), appearance);
        ui.separator();
        let colors = legend_colors(appearance);
        egui::Frame::new()
            .fill(colors.surface)
            .stroke(Stroke::new(1.0, colors.border))
            .corner_radius(7)
            .inner_margin(8)
            .show(ui, |ui| {
                ui.visuals_mut().override_text_color = Some(colors.text);
                ui.set_max_width(360.0);
                let content = |ui: &mut Ui| {
                    match appearance.legend_style {
                        LegendStyle::Compact => {
                            ui.horizontal_wrapped(|ui| {
                                for (label, family, _) in LEGEND_FAMILIES {
                                    // A chip allocates its full text width;
                                    // only the row can wrap, never its text.
                                    ui.add(
                                        egui::Button::new(
                                            egui::RichText::new(format!("● {label}"))
                                                .color(family.color()),
                                        )
                                        .sense(Sense::hover())
                                        .fill(colors.card)
                                        .stroke(Stroke::new(1.0, colors.border))
                                        .corner_radius(7)
                                        .wrap_mode(egui::TextWrapMode::Extend),
                                    );
                                }
                            });
                        }
                        LegendStyle::Detailed => {
                            for (label, family, _) in LEGEND_FAMILIES {
                                ui.horizontal(|ui| {
                                    ui.colored_label(family.color(), "●");
                                    ui.label(label);
                                });
                            }
                        }
                        LegendStyle::Cards => {
                            ui.set_min_width(310.0);
                            for (label, family, examples) in LEGEND_FAMILIES {
                                egui::Frame::new()
                                    .fill(colors.card)
                                    .stroke(Stroke::new(1.0, colors.border))
                                    .corner_radius(6)
                                    .inner_margin(8)
                                    .show(ui, |ui| {
                                        ui.set_min_width(290.0);
                                        ui.colored_label(
                                            family.color(),
                                            egui::RichText::new(format!("● {label}")).strong(),
                                        );
                                        ui.colored_label(
                                            colors.muted,
                                            egui::RichText::new(examples).monospace().small(),
                                        );
                                    });
                            }
                        }
                    }
                    ui.separator();
                    for (role, label) in [
                        (ColumnRole::Primary, "Clé primaire"),
                        (ColumnRole::Foreign, "Clé étrangère"),
                        (
                            ColumnRole::Unique,
                            "Membre d’une clé unique, éventuellement composite",
                        ),
                    ] {
                        ui.horizontal(|ui| {
                            let (r, _) =
                                ui.allocate_exact_size(egui::vec2(34.0, 16.0), Sense::hover());
                            paint_badge(ui.painter(), r.min, role, 1.0);
                            ui.label(label);
                        });
                    }
                };
                if appearance.legend_style == LegendStyle::Cards {
                    egui::ScrollArea::vertical()
                        .id_salt("legend_content")
                        .max_height(440.0)
                        .show(ui, content);
                } else {
                    content(ui);
                }
            });
    });
}

pub fn table_details(ui: &mut Ui, table: &Table) {
    ui.strong(table.id.label());
    egui::ScrollArea::vertical()
        .max_height(360.0)
        .show(ui, |ui| {
            for column in &table.columns {
                ui.horizontal(|ui| {
                    for role in roles(table, column) {
                        ui.colored_label(role.color(), role.label());
                    }
                    ui.label(&column.name);
                    ui.colored_label(type_family(&column.data_type).color(), &column.data_type);
                })
                .response
                .on_hover_text(tooltip(table, column));
            }
            for c in &table.constraints {
                ui.label(format!("{} · {}", c.name, c.definition));
            }
        });
}

#[cfg(test)]
mod legend_tests {
    use super::*;
    use kortex_infrastructure::storage::DiagramTheme;

    #[test]
    fn legend_palette_can_follow_diagram_or_remain_independent() {
        let graphite = Appearance::default();
        let emerald = Appearance {
            diagram_theme: DiagramTheme::Emerald,
            ..graphite
        };
        assert_ne!(
            legend_colors(graphite).surface,
            legend_colors(emerald).surface
        );
        for legend_palette in [LegendPalette::Neutral, LegendPalette::HighContrast] {
            assert_eq!(
                legend_colors(Appearance {
                    legend_palette,
                    ..graphite
                }),
                legend_colors(Appearance {
                    legend_palette,
                    ..emerald
                }),
            );
        }
        let contrast = legend_colors(Appearance {
            legend_palette: LegendPalette::HighContrast,
            ..graphite
        });
        assert_eq!(contrast.surface, Color32::BLACK);
        assert_eq!(contrast.text, Color32::WHITE);
    }
}
