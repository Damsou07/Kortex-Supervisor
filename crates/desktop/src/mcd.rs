//! A pure presentation surface: it can only edit local layout and request navigation.
use crate::theme;
use egui::{Align2, Color32, FontId, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, Vec2};
use kortex_domain::*;
use std::collections::BTreeSet;

pub struct DiagramView {
    pub zoom: f32,
    pub pan: Vec2,
    pub selected: BTreeSet<String>,
    pub selected_route: Option<String>,
    pub fit: bool,
    pub search: String,
    pub group_name: String,
    pub group_color: [u8; 4],
}
impl Default for DiagramView {
    fn default() -> Self {
        Self {
            zoom: 0.8,
            pan: Vec2::ZERO,
            selected: BTreeSet::new(),
            selected_route: None,
            fit: true,
            search: String::new(),
            group_name: "Nouveau domaine".into(),
            group_color: [110, 80, 25, 45],
        }
    }
}
#[derive(Default)]
pub struct DiagramAction {
    pub open_table: Option<String>,
    pub detach: Option<(String, Vec<String>)>,
    pub inspect: Option<String>,
}
const WIDTH: f32 = 280.0;
fn height(table: &Table) -> f32 {
    42.0 + table.columns.len() as f32 * 24.0 + 25.0
}
fn world_to_screen(p: Point, origin: Pos2, view: &DiagramView) -> Pos2 {
    origin + view.pan + egui::vec2(p.x, p.y) * view.zoom
}
fn screen_to_world(p: Pos2, origin: Pos2, view: &DiagramView) -> Point {
    let v = (p - origin - view.pan) / view.zoom;
    Point { x: v.x, y: v.y }
}
fn delta(ui: &Ui, zoom: f32) -> Point {
    let d = ui.input(|i| i.pointer.delta()) / zoom;
    Point { x: d.x, y: d.y }
}
fn table_rect(table: &Table, layout: &Diagram, origin: Pos2, view: &DiagramView) -> Rect {
    let p = layout
        .positions
        .get(&table.stable_id)
        .copied()
        .unwrap_or_default();
    Rect::from_min_size(
        world_to_screen(p, origin, view),
        egui::vec2(WIDTH, height(table)) * view.zoom,
    )
}
pub fn show(
    ui: &mut Ui,
    catalog: &Catalog,
    layout: &mut Diagram,
    history: &mut LayoutHistory,
    view: &mut DiagramView,
    subset: Option<&[String]>,
) -> DiagramAction {
    let mut action = DiagramAction::default();
    let visible: Vec<_> = catalog
        .tables
        .iter()
        .filter(|t| subset.is_none_or(|s| s.contains(&t.stable_id)))
        .collect();
    ui.horizontal_wrapped(|ui| {
        ui.label(theme::heading("MCD · STRUCTURE EN LECTURE SEULE"));
        if ui.button("Cadrer").clicked() {
            view.fit = true;
        }
        if ui.button("−").clicked() {
            view.zoom = (view.zoom / 1.2).max(0.12);
        }
        ui.label(format!("{:.0}%", view.zoom * 100.0));
        if ui.button("+").clicked() {
            view.zoom = (view.zoom * 1.2).min(2.5);
        }
        if ui
            .button("↶")
            .on_hover_text("Annuler le déplacement")
            .clicked()
        {
            history.undo(layout);
        }
        if ui
            .button("↷")
            .on_hover_text("Rétablir le déplacement")
            .clicked()
        {
            history.redo(layout);
        }
        if ui.button("Réorganiser").clicked() {
            history.record(layout);
            for t in &visible {
                layout.positions.remove(&t.stable_id);
            }
            layout.reconcile(&catalog.tables);
            view.fit = true;
        }
        if ui
            .add_enabled(
                !view.selected.is_empty(),
                egui::Button::new("Ouvrir la sélection ↗"),
            )
            .clicked()
        {
            action.detach = Some((
                "Sélection MCD".into(),
                view.selected.iter().cloned().collect(),
            ));
        }
    });
    ui.horizontal_wrapped(|ui| {
        ui.add(
            egui::TextEdit::singleline(&mut view.search)
                .hint_text("Rechercher une table…")
                .desired_width(170.0),
        );
        if ui.button("Trouver").clicked()
            && let Some(t) = visible.iter().find(|t| {
                t.id.label()
                    .to_lowercase()
                    .contains(&view.search.to_lowercase())
            })
        {
            view.selected = BTreeSet::from([t.stable_id.clone()]);
            view.fit = true;
            action.inspect = Some(t.stable_id.clone());
        }
        ui.separator();
        ui.add(
            egui::TextEdit::singleline(&mut view.group_name)
                .desired_width(130.0)
                .hint_text("Nom du groupe"),
        );
        ui.color_edit_button_srgba_unmultiplied(&mut view.group_color);
        if ui
            .add_enabled(
                !view.selected.is_empty(),
                egui::Button::new("Encadrer la sélection"),
            )
            .clicked()
        {
            history.record(layout);
            let members: Vec<_> = view.selected.iter().cloned().collect();
            let bounds = bounds(catalog, layout, Some(&members));
            let id = layout.groups.iter().map(|g| g.id).max().unwrap_or(0) + 1;
            layout.groups.push(Group {
                id,
                name: view.group_name.clone(),
                origin: Point {
                    x: bounds.min.x - 25.0,
                    y: bounds.min.y - 50.0,
                },
                size: Point {
                    x: bounds.width() + 50.0,
                    y: bounds.height() + 75.0,
                },
                members,
                color: view.group_color,
            });
        }
        ui.menu_button("Groupes", |ui| {
            let mut remove = None;
            for i in 0..layout.groups.len() {
                let before = layout.clone();
                let mut changed = false;
                ui.push_id(i, |ui| {
                    let group = &mut layout.groups[i];
                    changed |= ui.text_edit_singleline(&mut group.name).changed();
                    changed |= ui
                        .color_edit_button_srgba_unmultiplied(&mut group.color)
                        .changed();
                    ui.horizontal(|ui| {
                        if ui.button("↗ Fenêtre").clicked() {
                            action.detach = Some((group.name.clone(), group.members.clone()));
                            ui.close();
                        }
                        if ui.button("Sélectionner").clicked() {
                            view.selected = group.members.iter().cloned().collect();
                            view.fit = true;
                            ui.close();
                        }
                        if ui
                            .button("Remplacer les membres")
                            .on_hover_text("Utiliser les tables actuellement sélectionnées")
                            .clicked()
                        {
                            group.members = view.selected.iter().cloned().collect();
                            changed = true;
                        }
                        if ui.button("Supprimer le cadre").clicked() {
                            remove = Some(i);
                        }
                    });
                });
                if changed {
                    history.record(&before);
                }
            }
            if let Some(i) = remove {
                history.record(layout);
                layout.groups.remove(i);
            }
        });
    });
    ui.label(egui::RichText::new("Glisser : déplacer · Maj+clic : sélection multiple · Molette : zoom · Fond : déplacer la vue · Double-clic sur un lien : point de passage").small().color(theme::MUTED));
    let (canvas, background) = ui.allocate_exact_size(
        ui.available_size().max(egui::vec2(100.0, 100.0)),
        Sense::click_and_drag(),
    );
    let painter = ui.painter().with_clip_rect(canvas);
    painter.rect_filled(canvas, 0.0, Color32::from_rgb(11, 13, 17));
    if view.fit {
        let ids: Vec<_> = view.selected.iter().cloned().collect();
        let scope = if ids.is_empty() {
            subset
        } else {
            Some(ids.as_slice())
        };
        let b = bounds(catalog, layout, scope);
        view.zoom = ((canvas.width() - 60.0) / b.width().max(1.0))
            .min((canvas.height() - 60.0) / b.height().max(1.0))
            .clamp(0.12, 1.2);
        view.pan = canvas.size() / 2.0 - b.center().to_vec2() * view.zoom;
        view.fit = false;
    }
    if background.dragged_by(egui::PointerButton::Middle)
        || background.dragged_by(egui::PointerButton::Primary)
    {
        view.pan += ui.input(|i| i.pointer.delta());
    }
    if background.clicked() && !ui.input(|i| i.modifiers.shift) {
        view.selected.clear();
        view.selected_route = None;
    }
    if background.hovered()
        || canvas.contains(ui.input(|i| i.pointer.hover_pos().unwrap_or(Pos2::ZERO)))
    {
        let scroll = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll.abs() > 0.01 {
            let pos = ui
                .input(|i| i.pointer.hover_pos())
                .unwrap_or(canvas.center());
            let before = screen_to_world(pos, canvas.min, view);
            view.zoom = (view.zoom * (scroll * 0.002).exp()).clamp(0.12, 2.5);
            view.pan = pos - canvas.min - egui::vec2(before.x, before.y) * view.zoom;
        }
    }
    let grid_step = 40.0 * view.zoom;
    if grid_step >= 8.0 {
        let start = canvas.min
            + egui::vec2(
                view.pan.x.rem_euclid(grid_step),
                view.pan.y.rem_euclid(grid_step),
            );
        for x in 0..((canvas.width() / grid_step) as usize + 1) {
            for y in 0..((canvas.height() / grid_step) as usize + 1) {
                painter.circle_filled(
                    start + egui::vec2(x as f32 * grid_step, y as f32 * grid_step),
                    0.7,
                    Color32::from_gray(39),
                );
            }
        }
    }
    // Background rectangles never issue database commands.
    let groups = layout.groups.clone();
    for group in &groups {
        if subset.is_some_and(|s| !group.members.iter().any(|m| s.contains(m))) {
            continue;
        }
        let rect = Rect::from_min_size(
            world_to_screen(group.origin, canvas.min, view),
            egui::vec2(group.size.x, group.size.y) * view.zoom,
        );
        if !rect.intersects(canvas) {
            continue;
        }
        let color = Color32::from_rgba_unmultiplied(
            group.color[0],
            group.color[1],
            group.color[2],
            group.color[3],
        );
        painter.rect_filled(rect, 8.0, color);
        painter.rect_stroke(
            rect,
            8.0,
            Stroke::new(
                1.0,
                Color32::from_rgb(group.color[0], group.color[1], group.color[2]),
            ),
            StrokeKind::Inside,
        );
        painter.text(
            rect.min + egui::vec2(12.0, 14.0) * view.zoom,
            Align2::LEFT_TOP,
            &group.name,
            FontId::proportional(16.0 * view.zoom),
            theme::YELLOW,
        );
        let header = Rect::from_min_size(rect.min, egui::vec2(rect.width(), 35.0 * view.zoom));
        let response = ui.interact(
            header,
            egui::Id::unique(("group", group.id)),
            Sense::click_and_drag(),
        );
        if response.drag_started() {
            history.record(layout);
        }
        if response.dragged() {
            layout.move_group(group.id, delta(ui, view.zoom));
        }
        if response.double_clicked() {
            action.detach = Some((group.name.clone(), group.members.clone()));
        }
        let handle = Rect::from_min_size(rect.max - egui::vec2(14.0, 14.0), egui::vec2(14.0, 14.0));
        painter.line_segment(
            [handle.left_bottom(), handle.right_top()],
            Stroke::new(2.0, theme::YELLOW),
        );
        let response = ui.interact(
            handle,
            egui::Id::unique(("group_size", group.id)),
            Sense::drag(),
        );
        if response.drag_started() {
            history.record(layout);
        }
        if response.dragged() {
            let d = delta(ui, view.zoom);
            if let Some(g) = layout.groups.iter_mut().find(|g| g.id == group.id) {
                g.size.x = (g.size.x + d.x).max(100.0);
                g.size.y = (g.size.y + d.y).max(80.0);
            }
        }
    }
    for source in &visible {
        for fk in &source.foreign_keys {
            let Some(target) = catalog.tables.iter().find(|t| t.id == fk.target) else {
                continue;
            };
            let external = subset.is_some_and(|s| !s.contains(&target.stable_id));
            let route = layout.routes.get(&fk.id).cloned().unwrap_or_default();
            let src_rect = table_rect(source, layout, canvas.min, view);
            let dst_rect = table_rect(target, layout, canvas.min, view);
            let src_row = fk
                .columns
                .first()
                .and_then(|name| source.column_index(name))
                .unwrap_or(0);
            let dst_row = fk
                .target_columns
                .first()
                .and_then(|name| target.column_index(name))
                .unwrap_or(0);
            let start = egui::pos2(
                if route.source_left {
                    src_rect.left()
                } else {
                    src_rect.right()
                },
                src_rect.top()
                    + route
                        .source_y
                        .unwrap_or(48.0 + src_row as f32 * 24.0)
                        .clamp(0.0, height(source))
                        * view.zoom,
            );
            let end = if external {
                start + egui::vec2(90.0 * view.zoom, 0.0)
            } else {
                egui::pos2(
                    if route.target_right {
                        dst_rect.right()
                    } else {
                        dst_rect.left()
                    },
                    dst_rect.top()
                        + route
                            .target_y
                            .unwrap_or(48.0 + dst_row as f32 * 24.0)
                            .clamp(0.0, height(target))
                            * view.zoom,
                )
            };
            let mut points = vec![start];
            if !route.points.is_empty() {
                points.extend(
                    route
                        .points
                        .iter()
                        .map(|p| world_to_screen(*p, canvas.min, view)),
                );
            } else if source.id == target.id {
                points.extend([
                    start + egui::vec2(50.0, 0.0) * view.zoom,
                    egui::pos2(
                        start.x + 50.0 * view.zoom,
                        src_rect.top() - 30.0 * view.zoom,
                    ),
                    egui::pos2(end.x - 40.0 * view.zoom, src_rect.top() - 30.0 * view.zoom),
                    end - egui::vec2(40.0, 0.0) * view.zoom,
                ]);
            } else {
                let mid = (start.x + end.x) / 2.0;
                points.extend([egui::pos2(mid, start.y), egui::pos2(mid, end.y)]);
            }
            points.push(end);
            let selected = view.selected_route.as_deref() == Some(&fk.id);
            let color = if selected {
                theme::YELLOW
            } else {
                Color32::from_rgb(205, 66, 69)
            };
            for (segment, line) in points.windows(2).enumerate() {
                let bounds = Rect::from_two_pos(line[0], line[1]).expand(6.0);
                if !bounds.intersects(canvas) {
                    continue;
                }
                painter.line_segment(
                    [line[0], line[1]],
                    Stroke::new(if selected { 2.2 } else { 1.3 }, color),
                );
                let response = ui.interact(
                    bounds,
                    egui::Id::unique(("edge", &fk.id, segment)),
                    Sense::click(),
                );
                if response.clicked() {
                    view.selected_route = Some(fk.id.clone());
                }
                if response.double_clicked()
                    && let Some(pos) = response.interact_pointer_pos()
                {
                    history.record(layout);
                    let p = screen_to_world(pos, canvas.min, view);
                    let route = layout.routes.entry(fk.id.clone()).or_default();
                    if route.points.is_empty() {
                        route.points = points[1..points.len() - 1]
                            .iter()
                            .map(|p| screen_to_world(*p, canvas.min, view))
                            .collect();
                    }
                    route.points.insert(segment.min(route.points.len()), p);
                    view.selected_route = Some(fk.id.clone());
                }
                response.on_hover_text(format!(
                    "{} : {} → {}\nUPDATE {} · DELETE {}",
                    fk.name,
                    fk.columns.join(", "),
                    fk.target.label(),
                    fk.on_update,
                    fk.on_delete
                ));
            }
            let direction = (end - points[points.len() - 2]).normalized();
            let normal = egui::vec2(-direction.y, direction.x);
            painter.line_segment(
                [end, end - direction * 9.0 + normal * 4.0],
                Stroke::new(1.5, color),
            );
            painter.line_segment(
                [end, end - direction * 9.0 - normal * 4.0],
                Stroke::new(1.5, color),
            );
            let (reference, inverse) = cardinalities(source, fk);
            painter.text(
                start + egui::vec2(if route.source_left { -5.0 } else { 5.0 }, -13.0),
                if route.source_left {
                    Align2::RIGHT_BOTTOM
                } else {
                    Align2::LEFT_BOTTOM
                },
                inverse.label(),
                FontId::monospace((12.0 * view.zoom).max(9.0)),
                theme::YELLOW,
            );
            painter.text(
                end + egui::vec2(-5.0, -13.0),
                Align2::RIGHT_BOTTOM,
                if external {
                    format!("{} ↗ {}", reference.label(), target.id.name)
                } else {
                    reference.label().into()
                },
                FontId::monospace((12.0 * view.zoom).max(9.0)),
                theme::YELLOW,
            );
            if selected {
                for (i, p) in route.points.iter().enumerate() {
                    let screen = world_to_screen(*p, canvas.min, view);
                    painter.circle_filled(screen, 5.0, theme::YELLOW);
                    let r = ui.interact(
                        Rect::from_center_size(screen, egui::vec2(14.0, 14.0)),
                        egui::Id::unique(("waypoint", &fk.id, i)),
                        Sense::click_and_drag(),
                    );
                    if r.drag_started() {
                        history.record(layout);
                    }
                    if r.dragged() {
                        let d = delta(ui, view.zoom);
                        let p = &mut layout.routes.entry(fk.id.clone()).or_default().points[i];
                        p.x += d.x;
                        p.y += d.y;
                    }
                    if r.double_clicked() {
                        history.record(layout);
                        layout
                            .routes
                            .entry(fk.id.clone())
                            .or_default()
                            .points
                            .remove(i);
                        break;
                    }
                }
                for (is_source, p) in [(true, start), (false, end)] {
                    if external && !is_source {
                        continue;
                    }
                    painter.circle_filled(p, 4.0, theme::RED);
                    let r = ui.interact(
                        Rect::from_center_size(p, egui::vec2(12.0, 12.0)),
                        egui::Id::unique(("anchor", &fk.id, is_source)),
                        Sense::drag(),
                    );
                    if r.drag_started() {
                        history.record(layout);
                    }
                    if r.dragged()
                        && let Some(pointer) = r.interact_pointer_pos()
                    {
                        let route = layout.routes.entry(fk.id.clone()).or_default();
                        if is_source {
                            route.source_left = pointer.x < src_rect.center().x;
                            route.source_y = Some(
                                ((pointer.y - src_rect.top()) / view.zoom)
                                    .clamp(0.0, height(source)),
                            );
                        } else {
                            route.target_right = pointer.x > dst_rect.center().x;
                            route.target_y = Some(
                                ((pointer.y - dst_rect.top()) / view.zoom)
                                    .clamp(0.0, height(target)),
                            );
                        }
                    }
                }
            }
        }
    }
    for table in &visible {
        let rect = table_rect(table, layout, canvas.min, view);
        if !rect.intersects(canvas) {
            continue;
        }
        let selected = view.selected.contains(&table.stable_id);
        painter.rect_filled(rect, 5.0, theme::SURFACE);
        painter.rect_stroke(
            rect,
            5.0,
            Stroke::new(
                if selected { 2.0 } else { 1.0 },
                if selected {
                    theme::YELLOW
                } else {
                    Color32::from_gray(65)
                },
            ),
            StrokeKind::Inside,
        );
        let header = Rect::from_min_size(rect.min, egui::vec2(rect.width(), 36.0 * view.zoom));
        painter.rect_filled(
            header,
            4.0,
            if selected {
                Color32::from_rgb(67, 36, 31)
            } else {
                Color32::from_rgb(39, 32, 37)
            },
        );
        painter.text(
            header.min + egui::vec2(9.0, 10.0) * view.zoom,
            Align2::LEFT_TOP,
            &table.id.name,
            FontId::proportional(15.0 * view.zoom),
            Color32::WHITE,
        );
        for (i, col) in table.columns.iter().enumerate() {
            let p = rect.min + egui::vec2(9.0, 42.0 + i as f32 * 24.0) * view.zoom;
            if view.zoom > 0.26 {
                let label = format!("{}{}", theme::badges(table, col), col.name);
                let split = rect.left() + rect.width() * 0.68;
                painter
                    .with_clip_rect(
                        Rect::from_min_max(rect.min, egui::pos2(split, rect.bottom()))
                            .intersect(canvas),
                    )
                    .text(
                        p,
                        Align2::LEFT_TOP,
                        label,
                        FontId::monospace(11.0 * view.zoom),
                        if table.primary_key.contains(&col.name) {
                            theme::YELLOW
                        } else {
                            Color32::from_gray(220)
                        },
                    );
                painter
                    .with_clip_rect(
                        Rect::from_min_max(egui::pos2(split + 4.0, rect.top()), rect.max)
                            .intersect(canvas),
                    )
                    .text(
                        egui::pos2(rect.right() - 9.0 * view.zoom, p.y),
                        Align2::RIGHT_TOP,
                        &col.data_type,
                        FontId::monospace(10.0 * view.zoom),
                        theme::MUTED,
                    );
            }
        }
        painter.text(
            rect.left_bottom() + egui::vec2(9.0, -17.0) * view.zoom,
            Align2::LEFT_TOP,
            format!(
                "{} index · {} contraintes",
                table.indexes.len(),
                table.constraints.len()
            ),
            FontId::proportional(10.0 * view.zoom),
            theme::MUTED,
        );
        let response = ui.interact(
            header,
            egui::Id::unique(("table", &table.stable_id)),
            Sense::click_and_drag(),
        );
        if response.clicked() {
            if ui.input(|i| i.modifiers.shift) {
                if !view.selected.insert(table.stable_id.clone()) {
                    view.selected.remove(&table.stable_id);
                }
            } else {
                view.selected = BTreeSet::from([table.stable_id.clone()]);
            }
            action.inspect = Some(table.stable_id.clone());
        }
        if response.drag_started() {
            history.record(layout);
            if !view.selected.contains(&table.stable_id) {
                view.selected = BTreeSet::from([table.stable_id.clone()]);
            }
        }
        if response.dragged() {
            layout.move_tables(
                &view.selected.iter().cloned().collect::<Vec<_>>(),
                delta(ui, view.zoom),
            );
        }
        if response.double_clicked() {
            action.open_table = Some(table.stable_id.clone());
        }
        response.on_hover_text(format!(
            "{}\n{}\nDouble-clic : ouvrir les données",
            table.id.label(),
            table.comment
        ));
    }
    action
}
fn bounds(catalog: &Catalog, layout: &Diagram, subset: Option<&[String]>) -> Rect {
    let mut result = Rect::NOTHING;
    for table in &catalog.tables {
        if subset.is_some_and(|s| !s.contains(&table.stable_id)) {
            continue;
        }
        if let Some(p) = layout.positions.get(&table.stable_id) {
            result = result.union(Rect::from_min_size(
                egui::pos2(p.x, p.y),
                egui::vec2(WIDTH, height(table)),
            ));
        }
    }
    if result.is_finite() {
        result
    } else {
        Rect::from_min_size(Pos2::ZERO, egui::vec2(500.0, 400.0))
    }
}
