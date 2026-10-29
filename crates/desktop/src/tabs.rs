use crate::theme;
use egui::{Color32, RichText, Stroke, Ui};
pub struct TabResponse {
    pub activate: bool,
    pub close: bool,
}
pub fn document_tab(
    ui: &mut Ui,
    title: &str,
    connection: &str,
    active: bool,
    dirty: bool,
    reveal: bool,
) -> TabResponse {
    let mut result = TabResponse {
        activate: false,
        close: false,
    };
    let frame = egui::Frame::new()
        .inner_margin(egui::Margin::symmetric(5, 2))
        .corner_radius(5)
        .fill(if active {
            Color32::from_rgb(49, 38, 34)
        } else {
            theme::SURFACE
        })
        .stroke(Stroke::new(
            1.0,
            if active {
                theme::YELLOW
            } else {
                Color32::from_gray(52)
            },
        ));
    let response = frame
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.x = 3.0;
            ui.horizontal(|ui| {
                let label = title.to_owned();
                let width = ui
                    .painter()
                    .layout_no_wrap(
                        label.clone(),
                        egui::FontId::proportional(13.0),
                        Color32::WHITE,
                    )
                    .size()
                    .x
                    .clamp(24.0, 170.0);
                result.activate = ui
                    .add_sized(
                        [width, 22.0],
                        egui::Label::new(RichText::new(label).color(if active {
                            Color32::WHITE
                        } else {
                            theme::MUTED
                        }))
                        .truncate()
                        .sense(egui::Sense::click()),
                    )
                    .on_hover_text(format!("{connection} / {title}"))
                    .clicked();
                if dirty {
                    ui.colored_label(theme::YELLOW, "•")
                        .on_hover_text("Modifications en attente");
                }
                let close = ui.add_sized([22.0, 22.0], egui::Button::new("×").frame(false));
                let label = format!("Fermer l’onglet {title}");
                close.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &label));
                result.close = close.on_hover_text(label).clicked();
            });
        })
        .response;
    if reveal {
        response.scroll_to_me(Some(egui::Align::Center));
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui_kittest::{Harness, kittest::Queryable};
    #[test]
    fn tab_close_and_title_share_a_frame() {
        let mut initialized = false;
        let mut harness = Harness::builder().with_size([850.0, 100.0]).build_ui_state(
            move |ui, closed: &mut bool| {
                if !initialized {
                    theme::apply(ui.ctx());
                    initialized = true;
                }
                ui.horizontal(|ui| {
                    ui.push_id(0, |ui| {
                        document_tab(ui, "MCD", "Démonstration", false, false, false);
                    });
                    ui.push_id(1, |ui| {
                        let response = document_tab(
                            ui,
                            "commandes_avec_un_nom_très_long_à_tronquer",
                            "Démonstration",
                            true,
                            true,
                            false,
                        );
                        *closed |= response.close;
                    });
                    ui.push_id(2, |ui| {
                        document_tab(ui, "SQL 7", "Démonstration", false, false, false);
                    });
                });
            },
            false,
        );
        harness.run();
        harness.snapshot("tabs");
        assert!(harness.query_by_label("SQL 7").is_some());
        harness
            .get_by_label("Fermer l’onglet commandes_avec_un_nom_très_long_à_tronquer")
            .click();
        harness.run();
        assert!(*harness.state());
    }
}
