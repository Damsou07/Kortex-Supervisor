use egui_kittest::{Harness, kittest::Queryable};
use kortex_infrastructure::storage::{Appearance, DiagramTheme, LegendPalette, LegendStyle};
use kortex_supervisor::{columns, theme};

fn interactive_legend() -> Harness<'static, Appearance> {
    let mut initialized = false;
    Harness::builder().with_size([650.0, 800.0]).build_ui_state(
        move |ui, appearance: &mut Appearance| {
            if !initialized {
                theme::apply(ui.ctx());
                theme::set_appearance(ui.ctx(), *appearance);
                initialized = true;
            }
            theme::diagram_theme_selector(ui);
            columns::legend(ui);
            ui.add_space(660.0);
            let _ = ui.button("Hors du menu");
            *appearance = theme::appearance(ui.ctx());
        },
        Appearance::default(),
    )
}

#[test]
fn legend_style_and_palette_choices_stay_open_and_apply_immediately() {
    let mut harness = interactive_legend();
    harness.run();
    harness.get_by_label("Légende").click();
    harness.run();
    for (label, style) in [
        ("Détaillée", LegendStyle::Detailed),
        ("Cartes", LegendStyle::Cards),
        ("Compacte", LegendStyle::Compact),
    ] {
        harness.get_by_label(label).click();
        harness.run();
        assert_eq!(harness.state().legend_style, style);
        assert!(harness.query_by_label("Suivre le MCD").is_some());
    }
    for (label, palette) in [
        ("Neutre", LegendPalette::Neutral),
        ("Contraste élevé", LegendPalette::HighContrast),
        ("Suivre le MCD", LegendPalette::FollowDiagram),
    ] {
        harness.get_by_label(label).click();
        harness.run();
        assert_eq!(harness.state().legend_palette, palette);
        assert!(harness.query_by_label("Clé primaire").is_some());
    }
    harness.key_combination(&[egui::Key::Escape]);
    harness.run();
    assert!(harness.query_by_label("Clé primaire").is_none());
    harness.get_by_label("Légende").click();
    harness.run();
    harness.get_by_label("Hors du menu").click();
    harness.run();
    assert!(harness.query_by_label("Clé primaire").is_none());
}

#[test]
fn choosing_follow_diagram_uses_the_current_theme_after_an_independent_palette() {
    let mut harness = interactive_legend();
    harness.run();
    harness.get_by_label("Thème MCD").click();
    harness.run();
    harness.get_by_label("Émeraude").click();
    harness.run();
    assert_eq!(harness.state().diagram_theme, DiagramTheme::Emerald);
    harness.get_by_label("Légende").click();
    harness.run();
    harness.get_by_label("Neutre").click();
    harness.run();
    harness.get_by_label("Suivre le MCD").click();
    harness.run();
    assert_eq!(harness.state().legend_palette, LegendPalette::FollowDiagram);
    let expected = theme::palette_for(DiagramTheme::Emerald).canvas;
    assert!(harness.output().shapes.iter().any(|shape| {
        matches!(&shape.shape, egui::epaint::Shape::Rect(rect) if rect.fill == expected)
    }));
}

#[test]
fn compact_legend_keeps_each_chip_on_one_line_in_a_narrow_window() {
    let mut initialized = false;
    let mut harness = Harness::builder()
        .with_size([500.0, 600.0])
        .build_ui(move |ui| {
            if !initialized {
                theme::apply(ui.ctx());
                initialized = true;
            }
            columns::legend(ui);
        });
    harness.run();
    harness.get_by_label("Légende").click();
    harness.run();
    for label in [
        "● Nombres",
        "● Textes",
        "● Booléens",
        "● Dates et heures",
        "● UUID",
        "● JSON",
        "● Binaires",
        "● Autres types",
    ] {
        let text = harness
            .output()
            .shapes
            .iter()
            .find_map(|clipped| match &clipped.shape {
                egui::epaint::Shape::Text(text) if text.galley.job.text == label => {
                    Some((text, clipped.clip_rect))
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("missing visible chip {label}"));
        assert_eq!(
            text.0.galley.rows.len(),
            1,
            "chip {label} must not wrap its letters"
        );
        let bounds = text.0.galley.rect.translate(text.0.pos.to_vec2());
        assert!(
            bounds.height() < 24.0,
            "chip {label} is unexpectedly tall: {bounds:?}"
        );
        assert!(
            text.1.contains_rect(bounds),
            "chip {label} is clipped: {bounds:?} in {:?}",
            text.1
        );
    }
    for label in ["UQ", "Membre d’une clé unique, éventuellement composite"] {
        let (text, clip) = harness
            .output()
            .shapes
            .iter()
            .find_map(|clipped| match &clipped.shape {
                egui::epaint::Shape::Text(text) if text.galley.job.text == label => {
                    Some((text, clipped.clip_rect))
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("missing unique-key explanation {label}"));
        let bounds = text.galley.rect.translate(text.pos.to_vec2());
        assert!(
            clip.contains_rect(bounds),
            "unique-key explanation {label} is clipped: {bounds:?} in {clip:?}"
        );
    }
    harness.snapshot("legend_compact_narrow");
}
