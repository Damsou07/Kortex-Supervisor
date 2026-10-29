use egui::{Color32, RichText, Stroke};
pub const RED: Color32 = Color32::from_rgb(255, 48, 48);
pub const YELLOW: Color32 = Color32::from_rgb(255, 200, 48);
pub const MUTED: Color32 = Color32::from_rgb(145, 151, 164);
pub const SURFACE: Color32 = Color32::from_rgb(24, 26, 31);
pub fn apply(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "Noto Symbols".into(),
        egui::FontData::from_static(include_bytes!(
            "../../../assets/fonts/NotoSansSymbols2-Regular.ttf"
        ))
        .into(),
    );
    fonts.font_data.insert(
        "Noto Math".into(),
        egui::FontData::from_static(include_bytes!(
            "../../../assets/fonts/NotoSansMath-Regular.ttf"
        ))
        .into(),
    );
    for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
        fonts
            .families
            .entry(family)
            .or_default()
            .push("Noto Symbols".into());
    }
    for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
        fonts
            .families
            .entry(family)
            .or_default()
            .push("Noto Math".into());
    }
    ctx.set_fonts(fonts);
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = Color32::from_rgb(16, 18, 22);
    visuals.window_fill = SURFACE;
    visuals.extreme_bg_color = Color32::from_rgb(10, 12, 15);
    visuals.faint_bg_color = Color32::from_rgb(30, 33, 39);
    visuals.selection.bg_fill = Color32::from_rgb(95, 29, 35);
    visuals.selection.stroke = Stroke::new(1.0, RED);
    visuals.widgets.active.bg_stroke = Stroke::new(1.0, RED);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, YELLOW);
    visuals.override_text_color = Some(Color32::from_rgb(228, 231, 238));
    ctx.set_visuals(visuals);
    ctx.style_mut_of(egui::Theme::Dark, |style| {
        style.spacing.item_spacing = egui::vec2(8.0, 7.0);
        style.spacing.button_padding = egui::vec2(10.0, 5.0);
    });
}
pub fn heading(text: &str) -> RichText {
    RichText::new(text).color(YELLOW).strong().size(12.0)
}
pub fn badges(table: &kortex_domain::Table, column: &kortex_domain::Column) -> String {
    let mut result = String::new();
    if table.primary_key.contains(&column.name) {
        result.push_str("⚿ PK ");
    }
    if table
        .foreign_keys
        .iter()
        .any(|f| f.columns.contains(&column.name))
    {
        result.push_str("↗ FK ");
    }
    if table.unique_keys.iter().any(|k| k.contains(&column.name)) {
        result.push_str("◇ UQ ");
    }
    if column.nullable {
        result.push_str("○ NULL ");
    }
    if column.generated {
        result.push_str("ƒ ");
    }
    result
}
