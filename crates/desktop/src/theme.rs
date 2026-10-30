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
    pub text: Color32,
    pub muted: Color32,
    pub selection: Color32,
    pub cardinality: Color32,
    pub handle: Color32,
    pub selection_fill: Color32,
}

pub const DIAGRAM_THEMES: [(DiagramTheme, &str); 6] = [
    (DiagramTheme::Modern, "Graphite"),
    (DiagramTheme::Midnight, "Bleu nuit"),
    (DiagramTheme::Obsidian, "Obsidienne rouge"),
    (DiagramTheme::Emerald, "Émeraude"),
    (DiagramTheme::Amethyst, "Améthyste"),
    (DiagramTheme::HighContrast, "Contraste élevé"),
];

pub fn appearance(ctx: &egui::Context) -> Appearance {
    ctx.data(|data| data.get_temp(egui::Id::unique("kortex_appearance")))
        .unwrap_or_default()
}

pub fn set_appearance(ctx: &egui::Context, appearance: Appearance) {
    ctx.data_mut(|data| data.insert_temp(egui::Id::unique("kortex_appearance"), appearance));
}

pub fn diagram_palette(ctx: &egui::Context) -> DiagramPalette {
    palette_for(appearance(ctx).diagram_theme)
}

pub fn palette_for(choice: DiagramTheme) -> DiagramPalette {
    let graphite = DiagramPalette {
        canvas: Color32::from_rgb(11, 13, 17),
        grid: Color32::from_gray(39),
        card: SURFACE,
        border: Color32::from_gray(65),
        header: Color32::from_rgb(43, 31, 37),
        selected_header: Color32::from_rgb(78, 34, 39),
        relation: Color32::from_rgb(235, 72, 80),
        text: Color32::WHITE,
        muted: MUTED,
        selection: YELLOW,
        cardinality: YELLOW,
        handle: RED,
        selection_fill: Color32::from_rgba_unmultiplied(100, 160, 255, 30),
    };
    match choice {
        DiagramTheme::Modern => graphite,
        DiagramTheme::Midnight => DiagramPalette {
            canvas: Color32::from_rgb(9, 15, 27),
            grid: Color32::from_rgb(38, 51, 71),
            card: Color32::from_rgb(20, 29, 44),
            border: Color32::from_rgb(66, 84, 106),
            header: Color32::from_rgb(27, 43, 65),
            selected_header: Color32::from_rgb(76, 39, 57),
            relation: Color32::from_rgb(255, 88, 105),
            ..graphite
        },
        DiagramTheme::Obsidian => DiagramPalette {
            canvas: Color32::from_rgb(7, 8, 11),
            grid: Color32::from_rgb(38, 32, 37),
            card: Color32::from_rgb(23, 23, 29),
            border: Color32::from_rgb(82, 58, 67),
            header: Color32::from_rgb(44, 24, 33),
            selected_header: Color32::from_rgb(87, 23, 39),
            relation: Color32::from_rgb(255, 57, 76),
            selection: Color32::from_rgb(255, 112, 124),
            handle: Color32::from_rgb(255, 112, 124),
            selection_fill: Color32::from_rgba_unmultiplied(255, 57, 76, 30),
            ..graphite
        },
        DiagramTheme::Emerald => DiagramPalette {
            canvas: Color32::from_rgb(8, 19, 17),
            grid: Color32::from_rgb(31, 57, 50),
            card: Color32::from_rgb(20, 34, 32),
            border: Color32::from_rgb(57, 91, 79),
            header: Color32::from_rgb(25, 49, 43),
            selected_header: Color32::from_rgb(29, 74, 58),
            relation: Color32::from_rgb(61, 214, 166),
            selection: Color32::from_rgb(128, 242, 192),
            cardinality: Color32::from_rgb(218, 240, 164),
            handle: Color32::from_rgb(128, 242, 192),
            selection_fill: Color32::from_rgba_unmultiplied(61, 214, 166, 30),
            ..graphite
        },
        DiagramTheme::Amethyst => DiagramPalette {
            canvas: Color32::from_rgb(20, 13, 30),
            grid: Color32::from_rgb(53, 39, 69),
            card: Color32::from_rgb(32, 25, 44),
            border: Color32::from_rgb(85, 67, 108),
            header: Color32::from_rgb(49, 33, 65),
            selected_header: Color32::from_rgb(72, 42, 94),
            relation: Color32::from_rgb(180, 154, 255),
            selection: Color32::from_rgb(219, 191, 255),
            cardinality: Color32::from_rgb(231, 204, 255),
            handle: Color32::from_rgb(219, 191, 255),
            selection_fill: Color32::from_rgba_unmultiplied(180, 154, 255, 30),
            ..graphite
        },
        DiagramTheme::HighContrast => DiagramPalette {
            canvas: Color32::from_rgb(0, 0, 0),
            grid: Color32::from_gray(55),
            card: Color32::from_gray(10),
            border: Color32::WHITE,
            header: Color32::from_gray(26),
            selected_header: Color32::from_rgb(60, 47, 0),
            relation: Color32::WHITE,
            muted: Color32::from_gray(212),
            selection: Color32::from_rgb(255, 223, 64),
            cardinality: Color32::from_rgb(255, 223, 64),
            handle: Color32::from_rgb(255, 223, 64),
            selection_fill: Color32::from_rgba_unmultiplied(255, 223, 64, 38),
            ..graphite
        },
    }
}

pub fn diagram_theme_selector(ui: &mut egui::Ui) {
    ui.menu_button("Thème MCD", |ui| {
        let mut value = appearance(ui.ctx());
        for (theme, label) in DIAGRAM_THEMES {
            ui.selectable_value(&mut value.diagram_theme, theme, label);
        }
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
        style.spacing.scroll.bar_width = 4.0;
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
            ..Default::default()
        };
        set_appearance(&ctx, choice);
        assert_eq!(appearance(&ctx), choice);
        let midnight = diagram_palette(&ctx);
        assert_ne!(graphite.canvas, midnight.canvas);
        assert_ne!(graphite.card, midnight.card);
    }

    #[test]
    fn all_six_dark_palettes_keep_readable_card_and_header_text() {
        fn luminance(color: Color32) -> f32 {
            let linear = |channel: u8| {
                let value = channel as f32 / 255.0;
                if value <= 0.04045 {
                    value / 12.92
                } else {
                    ((value + 0.055) / 1.055).powf(2.4)
                }
            };
            0.2126 * linear(color.r()) + 0.7152 * linear(color.g()) + 0.0722 * linear(color.b())
        }
        let mut canvases = Vec::new();
        for (choice, label) in DIAGRAM_THEMES {
            let palette = palette_for(choice);
            assert!(
                !canvases.contains(&palette.canvas),
                "duplicate canvas: {label}"
            );
            canvases.push(palette.canvas);
            for surface in [
                palette.canvas,
                palette.card,
                palette.header,
                palette.selected_header,
            ] {
                let ratio = (luminance(palette.text) + 0.05) / (luminance(surface) + 0.05);
                assert!(
                    ratio >= 4.5,
                    "insufficient text contrast: {label} ({ratio})"
                );
            }
        }
        assert_eq!(canvases.len(), 6);
    }
}
