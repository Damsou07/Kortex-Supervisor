use egui::{Color32, RichText, Stroke};
use kortex_infrastructure::storage::{Appearance, DiagramTheme};
pub const RED: Color32 = Color32::from_rgb(255, 57, 66);
pub const YELLOW: Color32 = Color32::from_rgb(255, 200, 48);
pub const MUTED: Color32 = Color32::from_rgb(145, 151, 164);
pub const SURFACE: Color32 = Color32::from_rgb(24, 26, 31);

#[derive(Clone, Copy)]
pub struct DiagramPalette {
    pub canvas: Color32,
    pub grid: Color32,
    pub card: Color32,
    pub border: Color32,
    pub header: Color32,
    pub selected_header: Color32,
    pub relation: Color32,
}

pub fn appearance(ctx: &egui::Context) -> Appearance {
    ctx.data(|data| data.get_temp(egui::Id::unique("kortex_appearance")))
        .unwrap_or_default()
}

pub fn set_appearance(ctx: &egui::Context, appearance: Appearance) {
    ctx.data_mut(|data| data.insert_temp(egui::Id::unique("kortex_appearance"), appearance));
}

pub fn diagram_palette(ctx: &egui::Context) -> DiagramPalette {
    match appearance(ctx).diagram_theme {
        DiagramTheme::Modern => DiagramPalette {
            canvas: Color32::from_rgb(11, 13, 17),
            grid: Color32::from_gray(39),
            card: SURFACE,
            border: Color32::from_gray(65),
            header: Color32::from_rgb(43, 31, 37),
            selected_header: Color32::from_rgb(78, 34, 39),
            relation: Color32::from_rgb(235, 72, 80),
        },
        DiagramTheme::Midnight => DiagramPalette {
            canvas: Color32::from_rgb(9, 15, 27),
            grid: Color32::from_rgb(38, 51, 71),
            card: Color32::from_rgb(20, 29, 44),
            border: Color32::from_rgb(66, 84, 106),
            header: Color32::from_rgb(27, 43, 65),
            selected_header: Color32::from_rgb(76, 39, 57),
            relation: Color32::from_rgb(255, 88, 105),
        },
    }
}

pub fn diagram_theme_selector(ui: &mut egui::Ui) {
    ui.menu_button("Thème MCD", |ui| {
        let mut value = appearance(ui.ctx());
        ui.selectable_value(&mut value.diagram_theme, DiagramTheme::Modern, "Graphite");
        ui.selectable_value(
            &mut value.diagram_theme,
            DiagramTheme::Midnight,
            "Bleu nuit",
        );
        set_appearance(ui.ctx(), value);
    });
}
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
    visuals.panel_fill = Color32::from_rgb(17, 19, 24);
    visuals.window_fill = SURFACE;
    visuals.extreme_bg_color = Color32::from_rgb(10, 12, 15);
    visuals.faint_bg_color = Color32::from_rgb(33, 36, 43);
    visuals.selection.bg_fill = Color32::from_rgb(112, 32, 42);
    visuals.selection.stroke = Stroke::new(1.0, RED);
    visuals.widgets.active.bg_stroke = Stroke::new(1.0, RED);
    visuals.widgets.active.bg_fill = Color32::from_rgb(91, 30, 40);
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, YELLOW);
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(53, 45, 49);
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

#[cfg(test)]
mod tests {
    use super::*;
    use kortex_infrastructure::storage::LegendStyle;

    #[test]
    fn appearance_is_shared_and_palettes_are_distinct() {
        let ctx = egui::Context::default();
        let graphite = diagram_palette(&ctx);
        let choice = Appearance {
            diagram_theme: DiagramTheme::Midnight,
            legend_style: LegendStyle::Detailed,
        };
        set_appearance(&ctx, choice);
        assert_eq!(appearance(&ctx), choice);
        let midnight = diagram_palette(&ctx);
        assert_ne!(graphite.canvas, midnight.canvas);
        assert_ne!(graphite.card, midnight.card);
    }
}
