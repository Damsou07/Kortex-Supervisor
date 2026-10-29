use super::*;
pub(super) fn show(
    ui: &mut Ui,
    catalog: &Catalog,
    layout: &mut Diagram,
    history: &mut LayoutHistory,
    view: &mut DiagramView,
    action: &mut DiagramAction,
    subset: Option<&[String]>,
) {
    egui::Panel::right("mcd_groups")
        .default_size(280.0)
        .min_size(230.0)
        .resizable(true)
        .show(ui, |ui| {
            egui::ScrollArea::vertical()
                .id_salt("groups_panel_scroll")
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.heading("Groupes de tables");
                    if ui.button("+ Nouveau groupe").clicked() {
                        view.new_group();
                    }
                    if let Some(mut draft) = view.group_draft.take() {
                        ui.separator();
                        ui.strong(if draft.id.is_some() {
                            "Modifier le groupe"
                        } else {
                            "Créer un groupe"
                        });
                        ui.add(
                            egui::TextEdit::singleline(&mut draft.name)
                                .hint_text("Nom du groupe")
                                .desired_width(f32::INFINITY),
                        );
                        ui.horizontal(|ui| {
                            ui.label("Couleur");
                            ui.color_edit_button_srgba_unmultiplied(&mut draft.color);
                        });
                        ui.label(format!("{} table(s) sélectionnée(s)", draft.members.len()));
                        ui.label(
                            egui::RichText::new(
                                "Tracez un rectangle sur le MCD ; Maj ajoute à la sélection.",
                            )
                            .small()
                            .color(theme::MUTED),
                        );
                        if ui.button("Sélectionner sur le MCD").clicked() {
                            view.mode = InteractionMode::Select;
                            view.selected_route = None;
                        }
                        ui.add(
                            egui::TextEdit::singleline(&mut view.member_search)
                                .hint_text("Filtrer les tables…")
                                .desired_width(f32::INFINITY),
                        );
                        egui::ScrollArea::vertical()
                            .id_salt("group_members")
                            .max_height(180.0)
                            .show(ui, |ui| {
                                for table in &catalog.tables {
                                    if subset.is_some_and(|s| !s.contains(&table.stable_id))
                                        || !table
                                            .id
                                            .label()
                                            .to_lowercase()
                                            .contains(&view.member_search.to_lowercase())
                                    {
                                        continue;
                                    }
                                    let mut selected = draft.members.contains(&table.stable_id);
                                    if ui.checkbox(&mut selected, table.id.label()).changed() {
                                        if selected {
                                            view.selected.insert(table.stable_id.clone());
                                        } else {
                                            view.selected.remove(&table.stable_id);
                                        }
                                        draft
                                            .members
                                            .retain(|id| subset.is_some_and(|s| !s.contains(id)));
                                        draft.members.extend(view.selected.iter().cloned());
                                        draft.members.sort();
                                        draft.members.dedup();
                                    }
                                }
                            });
                        let (mut save, mut cancel) = (false, false);
                        ui.horizontal(|ui| {
                            save = ui
                                .add_enabled(
                                    draft.valid(catalog),
                                    egui::Button::new("Valider le groupe"),
                                )
                                .clicked();
                            cancel = ui.button("Annuler").clicked();
                        });
                        if save {
                            draft.save(catalog, layout, history);
                            view.finish_group();
                        } else if cancel {
                            view.finish_group();
                        } else {
                            view.group_draft = Some(draft);
                        }
                    } else {
                        ui.add(
                            egui::TextEdit::singleline(&mut view.group_search)
                                .hint_text("Rechercher un groupe…")
                                .desired_width(f32::INFINITY),
                        );
                        let groups = layout.groups.clone();
                        egui::ScrollArea::vertical()
                            .id_salt("groups_list")
                            .show(ui, |ui| {
                                for group in groups {
                                    if !group
                                        .name
                                        .to_lowercase()
                                        .contains(&view.group_search.to_lowercase())
                                    {
                                        continue;
                                    }
                                    ui.push_id(group.id, |ui| {
                                        ui.separator();
                                        ui.colored_label(
                                            Color32::from_rgb(
                                                group.color[0],
                                                group.color[1],
                                                group.color[2],
                                            ),
                                            egui::RichText::new(&group.name).strong(),
                                        );
                                        ui.label(format!("{} table(s)", group.members.len()));
                                        ui.horizontal_wrapped(|ui| {
                                            if ui.button("Sélectionner").clicked() {
                                                view.selected = group
                                                    .members
                                                    .iter()
                                                    .filter(|id| {
                                                        subset.is_none_or(|s| s.contains(id))
                                                    })
                                                    .cloned()
                                                    .collect();
                                                view.fit = true;
                                            }
                                            if ui.button("Modifier").clicked() {
                                                view.group_previous_mode = Some(view.mode);
                                                view.selected =
                                                    group.members.iter().cloned().collect();
                                                view.selected_route = None;
                                                view.mode = InteractionMode::Select;
                                                view.group_draft = Some(GroupDraft {
                                                    id: Some(group.id),
                                                    name: group.name.clone(),
                                                    members: group.members.clone(),
                                                    color: group.color,
                                                });
                                            }
                                            if ui.button("↗ Fenêtre").clicked() {
                                                action.detach = Some((
                                                    group.name.clone(),
                                                    group.members.clone(),
                                                ));
                                            }
                                            if ui.button("Supprimer le cadre").clicked() {
                                                remove_group(group.id, layout, history);
                                            }
                                        });
                                    });
                                }
                                if layout.groups.is_empty() {
                                    ui.label("Créez un groupe à partir d’une sélection de tables.");
                                }
                            });
                    }
                });
        });
}
