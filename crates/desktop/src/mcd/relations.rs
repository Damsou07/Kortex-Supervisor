use egui::{Rect, vec2};
use kortex_domain::{Catalog, ForeignKey, Table};
use std::collections::{HashMap, HashSet};

/// Collect each catalog relation once, including those crossing a partial view boundary.
pub(super) fn relations<'a>(
    catalog: &'a Catalog,
    subset: Option<&[String]>,
) -> Vec<(&'a Table, &'a Table, &'a ForeignKey, bool, bool)> {
    let visible = |table: &Table| subset.is_none_or(|s| s.contains(&table.stable_id));
    let targets: HashMap<_, _> = catalog
        .tables
        .iter()
        .map(|table| (&table.id, table))
        .collect();
    let mut result = Vec::new();
    for source in &catalog.tables {
        for fk in &source.foreign_keys {
            let Some(&target) = targets.get(&fk.target) else {
                continue;
            };
            let (source_visible, target_visible) = (visible(source), visible(target));
            if source_visible || target_visible {
                result.push((source, target, fk, source_visible, target_visible));
            }
        }
    }
    result
}

pub(super) fn overflow_description(
    source: &Table,
    target: &Table,
    fk: &ForeignKey,
    cardinality: &str,
    source_visible: bool,
    target_visible: bool,
) -> String {
    let direction = match (source_visible, target_visible) {
        (false, true) => "entrant",
        (true, false) => "sortant",
        _ => "interne",
    };
    format!(
        "{} · {} ({}) → {} ({}) · {} · {}",
        fk.name,
        source.id.label(),
        fk.columns.join(", "),
        target.id.label(),
        fk.target_columns.join(", "),
        direction,
        cardinality
    )
}

/// Deterministic screen-space packing backed by a spatial grid. The bounds are
/// the canvas, so labels do not capture space from off-screen geometry.
pub(super) fn place_labels(preferred: &[Rect], bounds: Rect) -> Vec<Rect> {
    const CELL: f32 = 48.0;
    fn cells(rect: Rect) -> impl Iterator<Item = (i32, i32)> {
        let min_x = (rect.left() / CELL).floor() as i32;
        let max_x = (rect.right() / CELL).floor() as i32;
        let min_y = (rect.top() / CELL).floor() as i32;
        let max_y = (rect.bottom() / CELL).floor() as i32;
        (min_x..=max_x).flat_map(move |x| (min_y..=max_y).map(move |y| (x, y)))
    }
    let mut placed: Vec<Rect> = Vec::with_capacity(preferred.len());
    let mut grid: HashMap<(i32, i32), Vec<usize>> = HashMap::new();
    let mut next_same: HashMap<(i32, i32, i32, i32), f32> = HashMap::new();
    let mut full: HashSet<(i32, i32, i32, i32)> = HashSet::new();
    for rect in preferred {
        let width = rect.width();
        let height = rect.height();
        let x = rect
            .left()
            .clamp(bounds.left(), (bounds.right() - width).max(bounds.left()));
        let y = rect
            .top()
            .clamp(bounds.top(), (bounds.bottom() - height).max(bounds.top()));
        let key = (
            x.round() as i32,
            y.round() as i32,
            width.round() as i32,
            height.round() as i32,
        );
        if width > bounds.width() || height > bounds.height() || full.contains(&key) {
            placed.push(Rect::NOTHING);
            continue;
        }
        let cached_y = *next_same.get(&key).unwrap_or(&y);
        let start_y = if cached_y + height <= bounds.bottom() {
            cached_y
        } else {
            y
        };
        let mut candidate = Rect::from_min_size(egui::pos2(x, start_y), rect.size());
        let mut column = 0;
        loop {
            let next = cells(candidate.expand(1.0))
                .filter_map(|cell| grid.get(&cell))
                .flatten()
                .map(|&index| placed[index])
                .filter(|other| candidate.expand(1.0).intersects(*other))
                .map(|other| other.bottom() + 2.0)
                .fold(candidate.top(), f32::max);
            if next <= candidate.top() {
                break;
            }
            candidate = candidate.translate(vec2(0.0, next - candidate.top()));
            if candidate.bottom() > bounds.bottom() {
                let step = width + 2.0;
                let limit = ((bounds.width() / step).ceil() as i32 + 1) * 2;
                let mut available = None;
                while column < limit {
                    column += 1;
                    let direction = if column % 2 == 1 { 1.0 } else { -1.0 };
                    let offset = ((column + 1) / 2) as f32 * step * direction;
                    let next_x = x + offset;
                    if next_x >= bounds.left() && next_x + width <= bounds.right() {
                        available = Some(next_x);
                        break;
                    }
                }
                if let Some(next_x) = available {
                    candidate = Rect::from_min_size(egui::pos2(next_x, bounds.top()), rect.size());
                } else {
                    // Signal overflow for the accessible list, without occupying canvas space.
                    full.insert(key);
                    candidate = Rect::NOTHING;
                    break;
                }
            }
        }
        if !candidate.is_finite() {
            placed.push(candidate);
            continue;
        }
        next_same.insert(key, candidate.bottom() + 2.0);
        let index = placed.len();
        for cell in cells(candidate) {
            grid.entry(cell).or_default().push(index);
        }
        placed.push(candidate);
    }
    placed
}

#[cfg(test)]
#[path = "relations_tests.rs"]
mod tests;
