//! Shared, display-only column metadata. SQL identifiers and declared types stay intact.
use crate::theme;
use egui::{Align2, Color32, FontId, Painter, Rect, Response, Sense, Stroke, Ui, Vec2};
use kortex_domain::{Column, Table};

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
            painter.circle_stroke(p(4.0, 6.0), 2.7 * scale, stroke);
            painter.line_segment([p(6.5, 6.0), p(12.0, 6.0)], stroke);
            painter.line_segment([p(10.0, 6.0), p(10.0, 9.0)], stroke);
        }
        ColumnRole::Foreign => {
            painter.line_segment([p(2.0, 9.0), p(11.0, 2.0)], stroke);
            painter.line_segment([p(6.0, 2.0), p(11.0, 2.0)], stroke);
            painter.line_segment([p(11.0, 2.0), p(11.0, 7.0)], stroke);
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
        Color32::from_gray(228),
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
            Stroke::new(scale.max(0.2), theme::MUTED),
        );
    }
    if column.generated {
        painter.text(
            egui::pos2(rect.right() - 7.0 * scale, rect.top() + 2.0 * scale),
            Align2::CENTER_TOP,
            "ƒ",
            FontId::proportional(10.0 * scale),
            theme::MUTED,
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

pub fn legend(ui: &mut Ui) {
    ui.menu_button("Légende", |ui| {
        for (label, family) in [
            ("Nombres", TypeFamily::Number),
            ("Textes", TypeFamily::Text),
            ("Booléens", TypeFamily::Boolean),
            ("Dates et heures", TypeFamily::Date),
            ("UUID", TypeFamily::Uuid),
            ("JSON", TypeFamily::Json),
            ("Binaires", TypeFamily::Binary),
            ("Autres types", TypeFamily::Other),
        ] {
            ui.colored_label(family.color(), label);
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
                let (r, _) = ui.allocate_exact_size(egui::vec2(34.0, 16.0), Sense::hover());
                paint_badge(ui.painter(), r.min, role, 1.0);
                ui.label(label);
            });
        }
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
