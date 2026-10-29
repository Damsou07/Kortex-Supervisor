use egui_kittest::{Harness, kittest::Queryable};
use kortex_supervisor::{columns, theme};

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
