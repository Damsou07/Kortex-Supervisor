//! A pure presentation surface: it can only edit local layout and request navigation.
mod groups;
mod relations;
use crate::{
    columns,
    diagram_cache::DiagramCache,
    shortcuts::{self, Command},
    theme,
};
use egui::{Align2, Color32, FontId, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, Vec2};
use kortex_application::diagram::{GroupDraft, move_group, move_tables, remove_group};
use kortex_domain::*;
use std::collections::BTreeSet;

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum InteractionMode {
    #[default]
    Pan,
    Select,
}

pub struct DiagramView {
    pub zoom: f32,
    pub pan: Vec2,
    pub selected: BTreeSet<String>,
    pub selected_route: Option<String>,
    pub fit: bool,
    pub search: String,
    pub mode: InteractionMode,
    pub canvas: Rect,
    pub groups_open: bool,
    pub group_draft: Option<GroupDraft>,
    cardinalities_open: bool,
    overflow_cardinalities: Vec<String>,
    group_search: String,
    member_search: String,
    selection_start: Option<Pos2>,
    selection_base: BTreeSet<String>,
    cache: DiagramCache,
    sizes: std::collections::BTreeMap<String, Point>,
    scope: Option<BTreeSet<String>>,
    group_previous_mode: Option<InteractionMode>,
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
            mode: InteractionMode::Pan,
            canvas: Rect::NOTHING,
            groups_open: false,
            group_draft: None,
            cardinalities_open: false,
            overflow_cardinalities: Vec::new(),
            group_search: String::new(),
            member_search: String::new(),
            selection_start: None,
            selection_base: BTreeSet::new(),
            cache: DiagramCache::default(),
            sizes: Default::default(),
            scope: None,
            group_previous_mode: None,
        }
    }
}
impl DiagramView {
    pub fn prepare_geometry(&mut self, catalog: &Catalog, layout: &Diagram) {
        self.cache.prepare(catalog, layout);
    }
    fn new_group(&mut self) {
        self.group_previous_mode.get_or_insert(self.mode);
        if let Some(scope) = &self.scope {
            self.selected.retain(|id| scope.contains(id));
        }
        self.groups_open = true;
        self.mode = InteractionMode::Select;
        self.selected_route = None;
        self.group_draft = Some(GroupDraft {
            name: "Nouveau groupe".into(),
            members: self.selected.iter().cloned().collect(),
            color: [110, 120, 200, 40],
            ..GroupDraft::default()
        });
    }
    fn sync_members(&mut self) {
        if let Some(draft) = &mut self.group_draft {
            draft
                .members
                .retain(|id| self.scope.as_ref().is_some_and(|s| !s.contains(id)));
            draft.members.extend(self.selected.iter().cloned());
            draft.members.sort();
            draft.members.dedup();
        }
    }
    fn finish_group(&mut self) {
        self.group_draft = None;
        self.mode = self.group_previous_mode.take().unwrap_or_default();
    }
}
#[derive(Default)]
pub struct DiagramAction {
    pub open_table: Option<String>,
    pub detach: Option<(String, Vec<String>)>,
    pub detach_full: bool,
    pub inspect: Option<String>,
}
const WIDTH: f32 = TABLE_WIDTH;
fn height(table: &Table, view: &DiagramView) -> f32 {
    view.sizes
        .get(&table.stable_id)
        .map(|s| s.y)
        .unwrap_or(67.0 + table.columns.len() as f32 * ROW_HEIGHT)
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
        egui::vec2(WIDTH, height(table, view)) * view.zoom,
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
    view.scope = subset.map(|s| s.iter().cloned().collect());
    view.selected
        .retain(|id| catalog.tables.iter().any(|t| &t.stable_id == id));
    let dragging = ui.input(|i| i.pointer.any_down());
    if view.cache.update(catalog, layout, history, dragging) {
        view.fit = true;
        view.selected.clear();
        view.selected_route = None;
    }
    view.sizes = LayoutInput::new(catalog, layout)
        .nodes
        .into_iter()
        .map(|n| (n.id, n.rect.size))
        .collect();
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
        ui.add_enabled_ui(!view.cache.busy() && view.group_draft.is_none(), |ui| {
            ui.menu_button("Disposition par défaut", |ui| {
                for (label, profile) in [
                    ("Compacte", LayoutProfile::Compact),
                    ("Espacée", LayoutProfile::Spacious),
                ] {
                    if ui.button(label).clicked() {
                        view.cache.arrange = Some((subset.map(|s| s.to_vec()), profile));
                        ui.close();
                    }
                }
            });
        });
        if view.cache.busy() {
            ui.spinner();
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
        if ui
            .button("Ouvrir le MCD ↗")
            .on_hover_text("Ouvrir tout le MCD dans une fenêtre maximisée")
            .clicked()
        {
            action.detach_full = true;
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
        if ui
            .selectable_label(view.mode == InteractionMode::Pan, "Déplacement")
            .on_hover_text("H · glisser le fond pour déplacer la vue")
            .clicked()
        {
            view.mode = InteractionMode::Pan;
        }
        if ui
            .selectable_label(view.mode == InteractionMode::Select, "Sélection")
            .on_hover_text("V · tracer un rectangle autour des tables")
            .clicked()
        {
            view.mode = InteractionMode::Select;
            view.selected_route = None;
        }
        if ui.selectable_label(view.groups_open, "Groupes").clicked() {
            view.groups_open = !view.groups_open;
        }
        theme::diagram_theme_selector(ui);
        columns::legend(ui);
        if ui
            .button("Cardinalités")
            .on_hover_text(format!(
                "{} cardinalités masquées sur le schéma",
                view.overflow_cardinalities.len()
            ))
            .clicked()
        {
            view.cardinalities_open = true;
        }
    });
    if view.groups_open {
        groups::show(ui, catalog, layout, history, view, &mut action, subset);
    }
    ui.label(egui::RichText::new("Glisser : déplacer · Maj+clic : sélection multiple · Molette : zoom · Fond : déplacer la vue · Double-clic sur un lien : point de passage").small().color(theme::MUTED));
    let (canvas, background) = ui.allocate_exact_size(
        ui.available_size().max(egui::vec2(100.0, 100.0)),
        Sense::click_and_drag(),
    );
    view.canvas = canvas;
    background.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Pane, true, "Zone du MCD"));
    if background.clicked() || background.drag_started() {
        background.request_focus();
    }
    if background.has_focus() {
        if shortcuts::consume(ui.ctx(), Command::SelectAll) {
            view.selected = visible.iter().map(|t| t.stable_id.clone()).collect();
            view.selected_route = None;
            view.sync_members();
        }
        if shortcuts::consume(ui.ctx(), Command::Undo) {
            history.undo(layout);
        }
        if shortcuts::consume(ui.ctx(), Command::Redo) {
            history.redo(layout);
        }
        if shortcuts::consume(ui.ctx(), Command::Fit) {
            view.fit = true;
        }
        if shortcuts::consume(ui.ctx(), Command::Select) {
            view.mode = InteractionMode::Select;
            view.selected_route = None;
        }
        if shortcuts::consume(ui.ctx(), Command::Pan) {
            view.mode = InteractionMode::Pan;
        }
        if shortcuts::consume(ui.ctx(), Command::Group) {
            view.new_group();
        }
        if shortcuts::consume(ui.ctx(), Command::Cancel) {
            view.selection_start = None;
            view.finish_group();
            view.selected_route = None;
        }
    }
    let painter = ui.painter().with_clip_rect(canvas);
    let palette = theme::diagram_palette(ui.ctx());
    let mut arrowheads = Vec::new();
    painter.rect_filled(canvas, 0.0, palette.canvas);
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
        || (view.mode == InteractionMode::Pan
            && background.dragged_by(egui::PointerButton::Primary))
    {
        view.pan += ui.input(|i| i.pointer.delta());
    }
    if background.clicked() && !ui.input(|i| i.modifiers.shift) {
        view.selected.clear();
        view.selected_route = None;
        view.sync_members();
    }
    if view.mode == InteractionMode::Select {
        if background.drag_started_by(egui::PointerButton::Primary) {
            view.selection_start = ui.input(|i| i.pointer.press_origin());
            view.selection_base = if ui.input(|i| i.modifiers.shift) {
                view.selected.clone()
            } else {
                BTreeSet::new()
            };
            view.selected_route = None;
        }
        if let Some(start) = view.selection_start
            && let Some(end) = ui.input(|i| i.pointer.interact_pos())
        {
            let selection = Rect::from_two_pos(start, end);
            view.selected = view.selection_base.clone();
            let hit: Vec<_> = visible
                .iter()
                .filter(|t| table_rect(t, layout, canvas.min, view).intersects(selection))
                .map(|t| t.stable_id.clone())
                .collect();
            view.selected.extend(hit);
            view.sync_members();
        }
        if background.drag_stopped() {
            view.selection_start = None;
        }
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
                    palette.grid,
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
            columns::ellipsize(
                &painter,
                &group.name,
                &FontId::proportional(16.0 * view.zoom),
                (rect.width() - 24.0 * view.zoom).max(0.0),
            ),
            FontId::proportional(16.0 * view.zoom),
            theme::YELLOW,
        );
        let header = Rect::from_min_size(rect.min, egui::vec2(rect.width(), 35.0 * view.zoom));
        let response = ui.interact(
            header.intersect(canvas),
            egui::Id::unique(("group", group.id)),
            if view.mode == InteractionMode::Pan {
                Sense::click_and_drag()
            } else {
                Sense::hover()
            },
        );
        if response.drag_started() {
            history.record(layout);
        }
        if response.dragged() {
            move_group(catalog, layout, group.id, delta(ui, view.zoom));
        }
        if response.double_clicked() {
            action.detach = Some((group.name.clone(), group.members.clone()));
        }
    }
    let mut cardinality_labels = Vec::new();
    for (source, target, fk, source_visible, target_visible) in
        relations::relations(catalog, subset)
    {
        let external = !source_visible || !target_visible;
        let src_rect = table_rect(source, layout, canvas.min, view);
        let dst_rect = table_rect(target, layout, canvas.min, view);
        let route = layout.routes.get(&fk.id).cloned().unwrap_or_else(|| {
            let Some(edge) = view.cache.geometry.edges.get(&fk.id) else {
                return Route::default();
            };
            let (Some(a), Some(b), Some(src), Some(dst)) = (
                edge.points.first(),
                edge.points.last(),
                view.cache.geometry.rects.get(&source.stable_id),
                view.cache.geometry.rects.get(&target.stable_id),
            ) else {
                return Route::default();
            };
            Route {
                source_left: a.x < src.origin.x + src.size.x / 2.0,
                target_right: b.x > dst.origin.x + dst.size.x / 2.0,
                source_y: Some(a.y - src.origin.y),
                target_y: Some(b.y - dst.origin.y),
                ..Route::default()
            }
        });
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
        let source_anchor = egui::pos2(
            if route.source_left {
                src_rect.left()
            } else {
                src_rect.right()
            },
            src_rect.top()
                + route
                    .source_y
                    .unwrap_or(48.0 + src_row as f32 * 24.0)
                    .clamp(0.0, height(source, view))
                    * view.zoom,
        );
        let target_anchor = egui::pos2(
            if route.target_right {
                dst_rect.right()
            } else {
                dst_rect.left()
            },
            dst_rect.top()
                + route
                    .target_y
                    .unwrap_or(48.0 + dst_row as f32 * 24.0)
                    .clamp(0.0, height(target, view))
                    * view.zoom,
        );
        // Boundary stubs point outward from the visible table port.
        let stub_length = 90.0 * view.zoom;
        let (start, end) = if !source_visible {
            (
                target_anchor
                    + egui::vec2(
                        if route.target_right {
                            stub_length
                        } else {
                            -stub_length
                        },
                        0.0,
                    ),
                target_anchor,
            )
        } else if !target_visible {
            (
                source_anchor,
                source_anchor
                    + egui::vec2(
                        if route.source_left {
                            -stub_length
                        } else {
                            stub_length
                        },
                        0.0,
                    ),
            )
        } else {
            (source_anchor, target_anchor)
        };
        let mut points = if !route.points.is_empty() {
            let mut p = vec![start];
            p.extend(
                route
                    .points
                    .iter()
                    .map(|p| world_to_screen(*p, canvas.min, view)),
            );
            p.push(end);
            p
        } else if let Some(edge) = view.cache.geometry.edges.get(&fk.id) {
            let mut p: Vec<_> = edge
                .points
                .iter()
                .map(|p| world_to_screen(*p, canvas.min, view))
                .collect();
            // Keep endpoints attached while a fresh routing job is pending.
            if let (Some(a), Some(b)) = (p.first().copied(), p.last().copied()) {
                let source_size = view.cache.geometry.rects.get(&source.stable_id);
                let target_size = view.cache.geometry.rects.get(&target.stable_id);
                let shift = |r: Option<&NodeRect>, t: &Table| {
                    r.and_then(|r| {
                        layout
                            .positions
                            .get(&t.stable_id)
                            .map(|p| egui::vec2(p.x - r.origin.x, p.y - r.origin.y) * view.zoom)
                    })
                    .unwrap_or_default()
                };
                let da = shift(source_size, source);
                let db = shift(target_size, target);
                p[0] = a + da;
                let last = p.len() - 1;
                p[last] = b + db;
            }
            p
        } else {
            let mid = (start.x + end.x) / 2.0;
            vec![start, egui::pos2(mid, start.y), egui::pos2(mid, end.y), end]
        };
        if external {
            points = vec![start, end];
        }
        let start = points.first().copied().unwrap_or(start);
        let end = points.last().copied().unwrap_or(end);
        let selected = view.selected_route.as_deref() == Some(&fk.id);
        let hovered = view.mode == InteractionMode::Pan
            && ui.input(|i| i.pointer.hover_pos()).is_some_and(|p| {
                canvas.contains(p)
                    && points
                        .windows(2)
                        .any(|s| distance_to_segment(p, s[0], s[1]) < 5.0)
            });
        let color = if selected || hovered {
            theme::YELLOW
        } else {
            palette.relation
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
                bounds.intersect(canvas),
                egui::Id::unique(("edge", &fk.id, segment)),
                if hovered && view.mode == InteractionMode::Pan {
                    Sense::click()
                } else {
                    Sense::hover()
                },
            );
            if response.clicked() {
                view.selected_route = Some(fk.id.clone());
            }
            if !external
                && response.double_clicked()
                && let Some(pos) = response.interact_pointer_pos()
            {
                history.record(layout);
                let p = screen_to_world(pos, canvas.min, view);
                let route = layout
                    .routes
                    .entry(fk.id.clone())
                    .or_insert_with(|| route.clone());
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
                "{} : {} ({}) → {} ({})\nUPDATE {} · DELETE {}",
                fk.name,
                source.id.label(),
                fk.columns.join(", "),
                fk.target.label(),
                fk.target_columns.join(", "),
                fk.on_update,
                fk.on_delete
            ));
        }
        if let Some(edge) = view.cache.geometry.edges.get(&fk.id) {
            for p in &edge.bridges {
                let center = world_to_screen(*p, canvas.min, view);
                if !points
                    .windows(2)
                    .any(|s| distance_to_segment(center, s[0], s[1]) < 0.1)
                {
                    continue;
                }
                let radius = (4.0 * view.zoom).clamp(0.8, 4.0);
                let horizontal = points.windows(2).any(|s| {
                    s[0].y == s[1].y
                        && (center.y - s[0].y).abs() < 0.1
                        && center.x > s[0].x.min(s[1].x)
                        && center.x < s[0].x.max(s[1].x)
                });
                painter.circle_filled(center, radius + 1.0, palette.canvas);
                let arc = (0..=8)
                    .map(|i| {
                        let a = std::f32::consts::PI * i as f32 / 8.0;
                        center
                            + if horizontal {
                                egui::vec2(a.cos(), -a.sin()) * radius
                            } else {
                                egui::vec2(a.sin(), a.cos()) * radius
                            }
                    })
                    .collect();
                painter.add(egui::Shape::line(arc, Stroke::new(1.3, color)));
            }
        }
        if points.len() > 1 {
            let direction = (end - points[points.len() - 2]).normalized();
            arrowheads.push((end, direction, color, (8.0 * view.zoom).clamp(0.8, 8.0)));
        }
        let (reference, inverse) = cardinalities(source, fk);
        let source_left = if source_visible {
            route.source_left
        } else {
            start.x < end.x
        };
        let target_right = if target_visible {
            route.target_right
        } else {
            end.x > start.x
        };
        cardinality_labels.push((
            start + egui::vec2(if source_left { -5.0 } else { 5.0 }, 5.0),
            if source_left {
                Align2::RIGHT_TOP
            } else {
                Align2::LEFT_TOP
            },
            if source_visible {
                inverse.label().into()
            } else {
                format!("{} → {}", source.id.name, inverse.label())
            },
            relations::overflow_description(
                source,
                target,
                fk,
                &format!("côté source : {}", inverse.label()),
                source_visible,
                target_visible,
            ),
        ));
        cardinality_labels.push((
            end + egui::vec2(if target_right { 5.0 } else { -5.0 }, -5.0),
            if target_right {
                Align2::LEFT_BOTTOM
            } else {
                Align2::RIGHT_BOTTOM
            },
            if target_visible {
                reference.label().into()
            } else {
                format!("{} → {}", reference.label(), target.id.name)
            },
            relations::overflow_description(
                source,
                target,
                fk,
                &format!("côté cible : {}", reference.label()),
                source_visible,
                target_visible,
            ),
        ));
        if selected && view.mode == InteractionMode::Pan && !external {
            for (i, p) in route.points.iter().enumerate() {
                let screen = world_to_screen(*p, canvas.min, view);
                let handle = Rect::from_center_size(screen, egui::vec2(14.0, 14.0));
                if !handle.intersects(canvas) {
                    continue;
                }
                painter.circle_filled(screen, 5.0, theme::YELLOW);
                let r = ui.interact(
                    handle.intersect(canvas),
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
                let handle = Rect::from_center_size(p, egui::vec2(12.0, 12.0));
                if !handle.intersects(canvas) {
                    continue;
                }
                painter.circle_filled(p, 4.0, theme::RED);
                let r = ui.interact(
                    handle.intersect(canvas),
                    egui::Id::unique(("anchor", &fk.id, is_source)),
                    Sense::drag(),
                );
                if r.drag_started() {
                    history.record(layout);
                }
                if r.dragged()
                    && let Some(pointer) = r.interact_pointer_pos()
                {
                    let route = layout
                        .routes
                        .entry(fk.id.clone())
                        .or_insert_with(|| route.clone());
                    if is_source {
                        route.source_left = pointer.x < src_rect.center().x;
                        route.source_y = Some(
                            ((pointer.y - src_rect.top()) / view.zoom)
                                .clamp(0.0, height(source, view)),
                        );
                    } else {
                        route.target_right = pointer.x > dst_rect.center().x;
                        route.target_y = Some(
                            ((pointer.y - dst_rect.top()) / view.zoom)
                                .clamp(0.0, height(target, view)),
                        );
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
        painter.rect_filled(rect, 5.0, palette.card);
        let header = Rect::from_min_size(rect.min, egui::vec2(rect.width(), 36.0 * view.zoom));
        painter.rect_filled(
            header,
            4.0,
            if selected {
                palette.selected_header
            } else {
                palette.header
            },
        );
        painter.rect_stroke(
            rect,
            5.0,
            Stroke::new(
                if selected { 2.0 } else { 1.0 },
                if selected {
                    theme::YELLOW
                } else {
                    palette.border
                },
            ),
            StrokeKind::Inside,
        );
        painter.text(
            header.min + egui::vec2(9.0, 10.0) * view.zoom,
            Align2::LEFT_TOP,
            columns::ellipsize(
                &painter,
                &table.id.name,
                &FontId::proportional(15.0 * view.zoom),
                (header.width() - 18.0 * view.zoom).max(0.0),
            ),
            FontId::proportional(15.0 * view.zoom),
            Color32::WHITE,
        );
        for (i, col) in table.columns.iter().enumerate() {
            let row = Rect::from_min_size(
                rect.min + egui::vec2(0.0, 42.0 + i as f32 * ROW_HEIGHT) * view.zoom,
                egui::vec2(rect.width(), ROW_HEIGHT * view.zoom),
            );
            if row.intersects(canvas) {
                columns::paint_row(&painter, row, table, col, view.zoom);
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
        let header_response = ui.interact(
            header.intersect(canvas),
            egui::Id::unique(("table_header", &table.stable_id)),
            if view.mode == InteractionMode::Pan {
                Sense::click_and_drag()
            } else {
                Sense::click()
            },
        );
        let body = Rect::from_min_max(egui::pos2(rect.left(), header.bottom()), rect.max);
        let body_response = ui.interact(
            body.intersect(canvas),
            egui::Id::unique(("table_body", &table.stable_id)),
            Sense::click(),
        );
        let response = header_response.union(body_response);
        if response.clicked() {
            background.request_focus();
            if ui.input(|i| i.modifiers.shift) {
                if !view.selected.insert(table.stable_id.clone()) {
                    view.selected.remove(&table.stable_id);
                }
            } else {
                view.selected = BTreeSet::from([table.stable_id.clone()]);
            }
            action.inspect = Some(table.stable_id.clone());
            view.sync_members();
        }
        if response.drag_started() {
            background.request_focus();
            history.record(layout);
            if !view.selected.contains(&table.stable_id) {
                view.selected = BTreeSet::from([table.stable_id.clone()]);
                view.sync_members();
            }
        }
        if response.dragged() {
            move_tables(
                catalog,
                layout,
                &view.selected.iter().cloned().collect::<Vec<_>>(),
                delta(ui, view.zoom),
            );
        }
        if response.double_clicked() {
            action.open_table = Some(table.stable_id.clone());
        }
        let pointer = ui.input(|i| i.pointer.hover_pos());
        response.on_hover_ui(|ui| {
            if view.zoom < 0.6 {
                columns::table_details(ui, table);
            } else if let Some(column) = pointer.and_then(|p| {
                table.columns.iter().enumerate().find_map(|(i, column)| {
                    let row = Rect::from_min_size(
                        rect.min + egui::vec2(0.0, 42.0 + i as f32 * ROW_HEIGHT) * view.zoom,
                        egui::vec2(rect.width(), ROW_HEIGHT * view.zoom),
                    );
                    row.contains(p).then_some(column)
                })
            }) {
                ui.label(columns::tooltip(table, column));
            } else {
                ui.label(format!(
                    "{}\n{}\nDouble-clic : ouvrir les données",
                    table.id.label(),
                    table.comment
                ));
            }
        });
    }
    for (end, direction, color, size) in arrowheads {
        let normal = egui::vec2(-direction.y, direction.x);
        painter.line_segment(
            [end, end - direction * size + normal * size * 0.45],
            Stroke::new(1.3, color),
        );
        painter.line_segment(
            [end, end - direction * size - normal * size * 0.45],
            Stroke::new(1.3, color),
        );
    }
    // Paint labels above table cards and keep those that cannot fit available in a list.
    let font = FontId::monospace((12.0 * view.zoom).max(9.0));
    let labels: Vec<_> = cardinality_labels
        .into_iter()
        .filter(|(anchor, _, _, _)| canvas.expand(20.0).contains(*anchor))
        .map(
            |(anchor, align, text, detail): (Pos2, Align2, String, String)| {
                let galley = painter.layout_no_wrap(text, font.clone(), theme::YELLOW);
                let rect = align.anchor_rect(Rect::from_min_size(anchor, galley.size()));
                (anchor, galley, rect, detail)
            },
        )
        .collect();
    let preferred: Vec<_> = labels.iter().map(|(_, _, rect, _)| *rect).collect();
    let mut overflow = Vec::new();
    for ((anchor, galley, preferred, detail), placed) in labels
        .into_iter()
        .zip(relations::place_labels(&preferred, canvas))
    {
        if !placed.is_finite() {
            overflow.push(detail);
            continue;
        }
        if placed != preferred {
            painter.line_segment([anchor, placed.center()], Stroke::new(0.8, theme::YELLOW));
        }
        // A background keeps text legible if a displaced label crosses a card.
        painter.rect_filled(placed.expand(1.0), 2.0, palette.canvas);
        painter.galley(placed.min, galley, theme::YELLOW);
    }
    if view.overflow_cardinalities != overflow {
        view.overflow_cardinalities = overflow;
        ui.ctx().request_repaint();
    }
    if view.cardinalities_open {
        egui::Window::new("Cardinalités masquées")
            .id(ui.scope_id().with("overflow_cardinalities"))
            .open(&mut view.cardinalities_open)
            .default_width(440.0)
            .show(ui.ctx(), |ui| {
                if view.overflow_cardinalities.is_empty() {
                    ui.label("Toutes les cardinalités visibles tiennent sur le schéma.");
                } else {
                    ui.label("Ces cardinalités ne tiennent pas sur le schéma à ce zoom :");
                    egui::ScrollArea::vertical()
                        .max_height(400.0)
                        .show(ui, |ui| {
                            for detail in &view.overflow_cardinalities {
                                ui.label(detail);
                                ui.separator();
                            }
                        });
                }
            });
    }
    if let Some(start) = view.selection_start
        && let Some(end) = ui.input(|i| i.pointer.interact_pos())
    {
        let selection = Rect::from_two_pos(start, end).intersect(canvas);
        painter.rect_filled(
            selection,
            0.0,
            Color32::from_rgba_unmultiplied(100, 160, 255, 30),
        );
        painter.rect_stroke(
            selection,
            0.0,
            Stroke::new(1.0, Color32::LIGHT_BLUE),
            StrokeKind::Inside,
        );
    }
    if view.cache.busy() {
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(16));
    }
    action
}
fn distance_to_segment(p: Pos2, a: Pos2, b: Pos2) -> f32 {
    let d = b - a;
    let t = if d.length_sq() > 0.0 {
        ((p - a).dot(d) / d.length_sq()).clamp(0.0, 1.0)
    } else {
        0.0
    };
    p.distance(a + d * t)
}
fn bounds(catalog: &Catalog, layout: &Diagram, subset: Option<&[String]>) -> Rect {
    let mut result = Rect::NOTHING;
    let sizes: std::collections::BTreeMap<_, _> = LayoutInput::new(catalog, layout)
        .nodes
        .into_iter()
        .map(|n| (n.id, n.rect.size))
        .collect();
    for table in &catalog.tables {
        if subset.is_some_and(|s| !s.contains(&table.stable_id)) {
            continue;
        }
        if let Some(p) = layout.positions.get(&table.stable_id) {
            result = result.union(Rect::from_min_size(
                egui::pos2(p.x, p.y),
                egui::vec2(WIDTH, sizes[&table.stable_id].y),
            ));
        }
    }
    if result.is_finite() {
        result
    } else {
        Rect::from_min_size(Pos2::ZERO, egui::vec2(500.0, 400.0))
    }
}
