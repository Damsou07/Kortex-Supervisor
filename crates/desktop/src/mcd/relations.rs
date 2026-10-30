use egui::{Pos2, Rect, Vec2, vec2};
use kortex_domain::{AnchorSide, Catalog, ForeignKey, Table};
use std::collections::HashMap;

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

pub(super) const BADGE_SIZE: Vec2 = vec2(40.0, 20.0);

/// Generated endpoint elbows are not user waypoints and must not shift insertion order.
pub(super) fn waypoint_insertion_index(
    points: &[Pos2],
    waypoints: &[Pos2],
    segment: usize,
) -> usize {
    let mut next = 0;
    for point in points.iter().take(segment + 1) {
        while waypoints
            .get(next)
            .is_some_and(|p| p.distance(*point) < 0.01)
        {
            next += 1;
        }
    }
    next
}

/// Fixed screen-space badge attached to the actual drawn endpoint.
pub(super) fn badge_rect(endpoint: Pos2, side: AnchorSide, source: bool) -> Rect {
    endpoint_label_rect(endpoint, side, source, BADGE_SIZE)
}

pub(super) fn endpoint_label_rect(
    endpoint: Pos2,
    side: AnchorSide,
    source: bool,
    size: Vec2,
) -> Rect {
    let n = side.normal();
    let normal = vec2(n.x, n.y);
    let tangent = vec2(-n.y, n.x) * if source { 1.0 } else { -1.0 };
    let half_normal = (normal.x.abs() * size.x + normal.y.abs() * size.y) / 2.0;
    let half_tangent = (tangent.x.abs() * size.x + tangent.y.abs() * size.y) / 2.0;
    Rect::from_center_size(
        endpoint + normal * (6.0 + half_normal) + tangent * (4.0 + half_tangent),
        size,
    )
}

/// Keep preferred positions or hide them. Callers supply priority order.
/// Table obstacles and accepted badges share the same spatial grid.
pub(super) fn place_labels(preferred: &[Rect], bounds: Rect, obstacles: &[Rect]) -> Vec<Rect> {
    const CELL: f32 = 64.0;
    fn cells(rect: Rect) -> impl Iterator<Item = (i32, i32)> {
        let min_x = (rect.left() / CELL).floor() as i32;
        let max_x = (rect.right() / CELL).floor() as i32;
        let min_y = (rect.top() / CELL).floor() as i32;
        let max_y = (rect.bottom() / CELL).floor() as i32;
        (min_x..=max_x).flat_map(move |x| (min_y..=max_y).map(move |y| (x, y)))
    }
    let mut occupied = Vec::new();
    let mut grid: HashMap<(i32, i32), Vec<usize>> = HashMap::new();
    for obstacle in obstacles.iter().filter(|r| r.intersects(bounds)) {
        let obstacle = obstacle.intersect(bounds);
        for cell in cells(obstacle) {
            grid.entry(cell).or_default().push(occupied.len());
        }
        occupied.push(obstacle);
    }
    preferred
        .iter()
        .map(|rect| {
            let padded = rect.expand(2.0);
            if !rect.is_finite()
                || !bounds.contains_rect(padded)
                || cells(padded)
                    .filter_map(|cell| grid.get(&cell))
                    .flatten()
                    .any(|&index| padded.intersects(occupied[index]))
            {
                return Rect::NOTHING;
            }
            for cell in cells(padded) {
                grid.entry(cell).or_default().push(occupied.len());
            }
            occupied.push(padded);
            *rect
        })
        .collect()
}

#[cfg(test)]
#[path = "relations_tests.rs"]
mod tests;
