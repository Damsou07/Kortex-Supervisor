//! Deterministic diagram geometry. Coordinates are independent of egui and SQL adapters.
use crate::*;
use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap, VecDeque};

pub const TABLE_WIDTH: f32 = 320.0;
pub const ROW_HEIGHT: f32 = 24.0;
pub const PORT_GAP: f32 = 16.0;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct NodeRect {
    pub origin: Point,
    pub size: Point,
}
impl NodeRect {
    pub fn right(self) -> f32 {
        self.origin.x + self.size.x
    }
    pub fn bottom(self) -> f32 {
        self.origin.y + self.size.y
    }
    pub fn intersects(&self, other: &Self) -> bool {
        self.origin.x < other.right()
            && self.right() > other.origin.x
            && self.origin.y < other.bottom()
            && self.bottom() > other.origin.y
    }
    pub fn expanded(self, amount: f32) -> Self {
        Self {
            origin: Point {
                x: self.origin.x - amount,
                y: self.origin.y - amount,
            },
            size: Point {
                x: self.size.x + 2.0 * amount,
                y: self.size.y + 2.0 * amount,
            },
        }
    }
    pub fn segment_crosses_interior(self, a: Point, b: Point) -> bool {
        if a.x == b.x {
            a.x > self.origin.x
                && a.x < self.right()
                && a.y.min(b.y) < self.bottom()
                && a.y.max(b.y) > self.origin.y
        } else if a.y == b.y {
            a.y > self.origin.y
                && a.y < self.bottom()
                && a.x.min(b.x) < self.right()
                && a.x.max(b.x) > self.origin.x
        } else {
            false
        }
    }
}

/// Resolve an anchor without changing its saved offset when a table becomes smaller.
pub fn resolve_table_anchor(rect: NodeRect, anchor: TableAnchor) -> Point {
    match anchor.side {
        AnchorSide::Left | AnchorSide::Right => Point {
            x: if anchor.side == AnchorSide::Left {
                rect.origin.x
            } else {
                rect.right()
            },
            y: rect.origin.y + anchor.offset.clamp(0.0, rect.size.y.max(0.0)),
        },
        AnchorSide::Top | AnchorSide::Bottom => Point {
            x: rect.origin.x + anchor.offset.clamp(0.0, rect.size.x.max(0.0)),
            y: if anchor.side == AnchorSide::Top {
                rect.origin.y
            } else {
                rect.bottom()
            },
        },
    }
}

/// Nearest point on the complete perimeter. Keeping the previous side only on
/// exact ties prevents corner jitter without locking a drag to that side.
pub fn project_table_anchor(
    rect: NodeRect,
    pointer: Point,
    preferred: Option<AnchorSide>,
) -> TableAnchor {
    [
        AnchorSide::Left,
        AnchorSide::Right,
        AnchorSide::Top,
        AnchorSide::Bottom,
    ]
    .into_iter()
    .map(|side| {
        let offset = match side {
            AnchorSide::Left | AnchorSide::Right => {
                (pointer.y - rect.origin.y).clamp(0.0, rect.size.y.max(0.0))
            }
            AnchorSide::Top | AnchorSide::Bottom => {
                (pointer.x - rect.origin.x).clamp(0.0, rect.size.x.max(0.0))
            }
        };
        let anchor = TableAnchor { side, offset };
        let point = resolve_table_anchor(rect, anchor);
        let distance = (point.x - pointer.x).powi(2) + (point.y - pointer.y).powi(2);
        (anchor, distance)
    })
    .min_by(|(a, da), (b, db)| {
        da.total_cmp(db)
            .then_with(|| (preferred != Some(a.side)).cmp(&(preferred != Some(b.side))))
    })
    .expect("four sides")
    .0
}

fn clear_path(points: &[Point], obstacles: &[NodeRect]) -> bool {
    points.windows(2).all(|p| {
        (p[0].x == p[1].x || p[0].y == p[1].y)
            && obstacles
                .iter()
                .all(|rect| !rect.segment_crosses_interior(p[0], p[1]))
    })
}

fn path_length(points: &[Point]) -> f32 {
    points
        .windows(2)
        .map(|p| (p[1].x - p[0].x).abs() + (p[1].y - p[0].y).abs())
        .sum()
}

fn consider_connector(
    best: &mut Option<(f32, Vec<Point>)>,
    points: Vec<Point>,
    obstacles: &[NodeRect],
) {
    if clear_path(&points, obstacles) {
        let length = path_length(&points);
        if best.as_ref().is_none_or(|(previous, _)| length < *previous) {
            *best = Some((length, points));
        }
    }
}

/// Common drags need only one or two elbows. The visibility-grid fallback is
/// used only when surrounding tables obstruct every short connector.
fn connect_orthogonal(start: Point, end: Point, obstacles: &[NodeRect]) -> Vec<Point> {
    if start == end {
        return vec![start];
    }
    let mut best = None;
    consider_connector(
        &mut best,
        vec![
            start,
            Point {
                x: end.x,
                y: start.y,
            },
            end,
        ],
        obstacles,
    );
    consider_connector(
        &mut best,
        vec![
            start,
            Point {
                x: start.x,
                y: end.y,
            },
            end,
        ],
        obstacles,
    );
    if let Some((_, points)) = best {
        return simplify_path(points);
    }
    let mut xs = vec![start.x, end.x];
    let mut ys = vec![start.y, end.y];
    for rect in obstacles {
        xs.extend([rect.origin.x - 12.0, rect.right() + 12.0]);
        ys.extend([rect.origin.y - 12.0, rect.bottom() + 12.0]);
    }
    for &x in &xs {
        consider_connector(
            &mut best,
            vec![start, Point { x, y: start.y }, Point { x, y: end.y }, end],
            obstacles,
        );
    }
    for &y in &ys {
        consider_connector(
            &mut best,
            vec![start, Point { x: start.x, y }, Point { x: end.x, y }, end],
            obstacles,
        );
    }
    if let Some((_, points)) = best {
        return simplify_path(points);
    }
    // Preserve narrow corridors where the normal clearance on both sides
    // would otherwise place every visibility coordinate inside an obstacle.
    for rect in obstacles {
        xs.extend([rect.origin.x, rect.right()]);
        ys.extend([rect.origin.y, rect.bottom()]);
    }
    visibility_path(start, end, xs, ys, obstacles)
}

fn visibility_path(
    start: Point,
    end: Point,
    mut xs: Vec<f32>,
    mut ys: Vec<f32>,
    obstacles: &[NodeRect],
) -> Vec<Point> {
    xs.sort_by(f32::total_cmp);
    xs.dedup();
    ys.sort_by(f32::total_cmp);
    ys.dedup();
    let index = |point: Point| {
        ys.iter().position(|y| *y == point.y).expect("known y") * xs.len()
            + xs.iter().position(|x| *x == point.x).expect("known x")
    };
    let point = |i: usize| Point {
        x: xs[i % xs.len()],
        y: ys[i / xs.len()],
    };
    let (first, last) = (index(start), index(end));
    let mut costs = vec![f32::INFINITY; xs.len() * ys.len()];
    let mut previous = vec![usize::MAX; costs.len()];
    let mut queue = BinaryHeap::new();
    costs[first] = 0.0;
    queue.push(Reverse((0_u32, 0_u32, first)));
    while let Some(Reverse((_, bits, i))) = queue.pop() {
        let cost = f32::from_bits(bits);
        if cost > costs[i] {
            continue;
        }
        if i == last {
            let mut path = vec![end];
            let mut at = i;
            while previous[at] != usize::MAX {
                at = previous[at];
                path.push(point(at));
            }
            path.reverse();
            return simplify_path(path);
        }
        let p = point(i);
        let (x, y) = (i % xs.len(), i / xs.len());
        for (valid, next) in [
            (x > 0, i.saturating_sub(1)),
            (x + 1 < xs.len(), i + 1),
            (y > 0, i.saturating_sub(xs.len())),
            (y + 1 < ys.len(), i + xs.len()),
        ] {
            if !valid {
                continue;
            }
            let q = point(next);
            if !clear_path(&[p, q], obstacles) {
                continue;
            }
            let next_cost = cost + (p.x - q.x).abs() + (p.y - q.y).abs();
            if next_cost < costs[next] {
                costs[next] = next_cost;
                previous[next] = i;
                let estimate = next_cost + (q.x - end.x).abs() + (q.y - end.y).abs();
                queue.push(Reverse((estimate.to_bits(), next_cost.to_bits(), next)));
            }
        }
    }
    // An explicit user waypoint can lie inside a table. Preserve that position
    // rather than silently moving the user's route to manufacture a valid path.
    vec![
        start,
        Point {
            x: end.x,
            y: start.y,
        },
        end,
    ]
}

/// Reconnect a full route to current table anchors without moving its interior
/// waypoints. Used by both manual routing and live drag previews; no worker wait.
pub fn reconnect_route_endpoints(
    points: &[Point],
    source: (NodeRect, TableAnchor),
    target: (NodeRect, TableAnchor),
    obstacles: &[NodeRect],
) -> Vec<Point> {
    let start = resolve_table_anchor(source.0, source.1);
    let end = resolve_table_anchor(target.0, target.1);
    let outward = |point: Point, side: AnchorSide, distance: f32| {
        let normal = side.normal();
        Point {
            x: point.x + normal.x * distance,
            y: point.y + normal.y * distance,
        }
    };
    let stub = |point: Point, side: AnchorSide| {
        let mut distance = 24.0_f32;
        for rect in obstacles {
            let gap = match side {
                AnchorSide::Right
                    if point.y > rect.origin.y
                        && point.y < rect.bottom()
                        && rect.origin.x >= point.x =>
                {
                    Some(rect.origin.x - point.x)
                }
                AnchorSide::Left
                    if point.y > rect.origin.y
                        && point.y < rect.bottom()
                        && rect.right() <= point.x =>
                {
                    Some(point.x - rect.right())
                }
                AnchorSide::Bottom
                    if point.x > rect.origin.x
                        && point.x < rect.right()
                        && rect.origin.y >= point.y =>
                {
                    Some(rect.origin.y - point.y)
                }
                AnchorSide::Top
                    if point.x > rect.origin.x
                        && point.x < rect.right()
                        && rect.bottom() <= point.y =>
                {
                    Some(point.y - rect.bottom())
                }
                _ => None,
            };
            if let Some(gap) = gap {
                distance = distance.min(gap * 0.5);
            }
        }
        outward(point, side, distance)
    };
    let (a, b) = (stub(start, source.1.side), stub(end, target.1.side));
    let interior = if points.len() > 2 {
        &points[1..points.len() - 1]
    } else {
        &[]
    };
    if start == end && source.1.side == target.1.side && interior.is_empty() {
        let normal = source.1.side.normal();
        for direction in [1.0, -1.0] {
            let tangent = Point {
                x: -normal.y * PORT_GAP * direction,
                y: normal.x * PORT_GAP * direction,
            };
            let stub_length = (a.x - start.x).abs() + (a.y - start.y).abs();
            let far = outward(a, source.1.side, PORT_GAP.min(stub_length * 0.5));
            let loop_path = vec![
                start,
                a,
                Point {
                    x: a.x + tangent.x,
                    y: a.y + tangent.y,
                },
                Point {
                    x: far.x + tangent.x,
                    y: far.y + tangent.y,
                },
                far,
                end,
            ];
            if clear_path(&loop_path, obstacles) {
                return loop_path;
            }
        }
    }
    let mut result = vec![start, a];
    if let (Some(&first), Some(&last)) = (interior.first(), interior.last()) {
        result.extend(connect_orthogonal(a, first, obstacles).into_iter().skip(1));
        result.extend_from_slice(&interior[1..]);
        result.extend(connect_orthogonal(last, b, obstacles).into_iter().skip(1));
    } else {
        result.extend(connect_orthogonal(a, b, obstacles).into_iter().skip(1));
    }
    result.push(end);
    result.dedup();
    result
}
#[derive(Clone, Debug, PartialEq)]
pub struct LayoutNode {
    pub id: String,
    pub rect: NodeRect,
}
#[derive(Clone, Debug, PartialEq)]
pub struct LayoutEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    pub source_y: f32,
    pub target_y: f32,
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LayoutInput {
    pub nodes: Vec<LayoutNode>,
    pub edges: Vec<LayoutEdge>,
    pub groups: Vec<Vec<String>>,
    pub fixed: BTreeSet<String>,
    pub manual: BTreeMap<String, Route>,
}
impl LayoutInput {
    pub fn new(catalog: &Catalog, diagram: &Diagram) -> Self {
        let ids: BTreeMap<_, _> = catalog.tables.iter().map(|t| (&t.id, t)).collect();
        let mut edges: Vec<_> = catalog
            .tables
            .iter()
            .flat_map(|t| {
                t.foreign_keys.iter().filter_map(|fk| {
                    ids.get(&fk.target).map(|target| LayoutEdge {
                        id: fk.id.clone(),
                        source: t.stable_id.clone(),
                        target: target.stable_id.clone(),
                        source_y: 48.0
                            + fk.columns
                                .first()
                                .and_then(|c| t.column_index(c))
                                .unwrap_or(0) as f32
                                * ROW_HEIGHT,
                        target_y: 48.0
                            + fk.target_columns
                                .first()
                                .and_then(|c| target.column_index(c))
                                .unwrap_or(0) as f32
                                * ROW_HEIGHT,
                    })
                })
            })
            .collect();
        edges.sort_by(|a, b| a.id.cmp(&b.id));
        let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
        for edge in &edges {
            *counts.entry(&edge.source).or_default() += 1;
            *counts.entry(&edge.target).or_default() += 1;
        }
        let mut nodes: Vec<_> = catalog
            .tables
            .iter()
            .map(|table| {
                let rows = 67.0 + table.columns.len() as f32 * ROW_HEIGHT;
                let ports = 64.0
                    + counts.get(table.stable_id.as_str()).copied().unwrap_or(0) as f32 * PORT_GAP;
                LayoutNode {
                    id: table.stable_id.clone(),
                    rect: NodeRect {
                        origin: diagram
                            .positions
                            .get(&table.stable_id)
                            .copied()
                            .unwrap_or_default(),
                        size: Point {
                            x: TABLE_WIDTH,
                            y: (rows.max(ports) / 16.0).ceil() * 16.0,
                        },
                    },
                }
            })
            .collect();
        nodes.sort_by(|a, b| a.id.cmp(&b.id));
        Self {
            nodes,
            edges,
            groups: diagram.groups.iter().map(|g| g.members.clone()).collect(),
            manual: diagram.routes.clone(),
            ..Self::default()
        }
    }
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct EdgeGeometry {
    pub points: Vec<Point>,
    pub bridges: Vec<Point>,
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LayoutResult {
    pub rects: BTreeMap<String, NodeRect>,
    pub edges: BTreeMap<String, EdgeGeometry>,
}
impl LayoutResult {
    pub fn tables_in(&self, rect: NodeRect) -> Vec<String> {
        self.rects
            .iter()
            .filter(|(_, r)| r.intersects(&rect))
            .map(|(id, _)| id.clone())
            .collect()
    }
}

/// Strongly connected components become layers; cycles never require deleting edges.
fn ranks(n: usize, edges: &[(usize, usize)]) -> Vec<usize> {
    struct Tarjan<'a> {
        graph: &'a [Vec<usize>],
        next: usize,
        index: Vec<usize>,
        low: Vec<usize>,
        stack: Vec<usize>,
        active: Vec<bool>,
        component: Vec<usize>,
        count: usize,
    }
    impl Tarjan<'_> {
        fn visit(&mut self, v: usize) {
            self.next += 1;
            self.index[v] = self.next;
            self.low[v] = self.next;
            self.stack.push(v);
            self.active[v] = true;
            for &w in &self.graph[v] {
                if self.index[w] == 0 {
                    self.visit(w);
                    self.low[v] = self.low[v].min(self.low[w]);
                } else if self.active[w] {
                    self.low[v] = self.low[v].min(self.index[w]);
                }
            }
            if self.low[v] == self.index[v] {
                while let Some(w) = self.stack.pop() {
                    self.active[w] = false;
                    self.component[w] = self.count;
                    if w == v {
                        break;
                    }
                }
                self.count += 1;
            }
        }
    }
    let mut graph = vec![vec![]; n];
    for &(a, b) in edges {
        graph[a].push(b);
    }
    let mut t = Tarjan {
        graph: &graph,
        next: 0,
        index: vec![0; n],
        low: vec![0; n],
        stack: vec![],
        active: vec![false; n],
        component: vec![0; n],
        count: 0,
    };
    for v in 0..n {
        if t.index[v] == 0 {
            t.visit(v);
        }
    }
    let mut dag = vec![BTreeSet::new(); t.count];
    let mut degree = vec![0; t.count];
    for &(a, b) in edges {
        let (a, b) = (t.component[a], t.component[b]);
        if a != b && dag[a].insert(b) {
            degree[b] += 1;
        }
    }
    let mut queue: VecDeque<_> = (0..t.count).filter(|&i| degree[i] == 0).collect();
    let mut rank = vec![0; t.count];
    while let Some(a) = queue.pop_front() {
        for &b in &dag[a] {
            rank[b] = rank[b].max(rank[a] + 1);
            degree[b] -= 1;
            if degree[b] == 0 {
                queue.push_back(b);
            }
        }
    }
    t.component.iter().map(|&c| rank[c]).collect()
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LayoutProfile {
    #[default]
    Compact,
    Spacious,
}
impl LayoutProfile {
    fn spacing(self) -> f32 {
        match self {
            Self::Compact => 1.0,
            Self::Spacious => 2.0,
        }
    }
}

fn layered(nodes: &[LayoutNode], edges: &[(usize, usize)], spacing: f32) -> Vec<Point> {
    if nodes.is_empty() {
        return vec![];
    }
    let rank = ranks(nodes.len(), edges);
    let mut layers = vec![vec![]; rank.iter().max().copied().unwrap_or(0) + 1];
    for (i, &r) in rank.iter().enumerate() {
        layers[r].push(i);
    }
    let mut order = vec![0.0; nodes.len()];
    for pass in 0..8 {
        for layer in &layers {
            for (i, &node) in layer.iter().enumerate() {
                order[node] = i as f32;
            }
        }
        let sequence: Vec<_> = if pass % 2 == 0 {
            (0..layers.len()).collect()
        } else {
            (0..layers.len()).rev().collect()
        };
        for l in sequence {
            layers[l].sort_by(|&a, &b| {
                let bary = |v| {
                    let near: Vec<_> = edges
                        .iter()
                        .filter_map(|&(s, t)| {
                            if pass % 2 == 0 && t == v && rank[s] < l {
                                Some(order[s])
                            } else if pass % 2 == 1 && s == v && rank[t] > l {
                                Some(order[t])
                            } else {
                                None
                            }
                        })
                        .collect();
                    if near.is_empty() {
                        order[v]
                    } else {
                        near.iter().sum::<f32>() / near.len() as f32
                    }
                };
                bary(a)
                    .total_cmp(&bary(b))
                    .then_with(|| nodes[a].id.cmp(&nodes[b].id))
            });
        }
    }
    let mut result = vec![Point::default(); nodes.len()];
    let area: f32 = nodes
        .iter()
        .map(|n| (n.rect.size.x + 48.0 * spacing) * (n.rect.size.y + 32.0 * spacing))
        .sum();
    let target_height = (area / 1.6)
        .sqrt()
        .max(nodes.iter().map(|n| n.rect.size.y).fold(0.0, f32::max));
    let target_width = (area * 1.6).sqrt().max(1200.0);
    let (mut x, mut band_y, mut band_height) = (0.0, 0.0, 0.0_f32);
    for layer in &layers {
        let (mut local_x, mut y, mut width, mut height) = (0.0, 0.0, 0.0_f32, 0.0_f32);
        let mut local = vec![];
        for &i in layer {
            if y > 0.0 && y + nodes[i].rect.size.y > target_height {
                local_x += width + 48.0 * spacing;
                y = 0.0;
                width = 0.0;
            }
            local.push((i, Point { x: local_x, y }));
            y += nodes[i].rect.size.y + 32.0 * spacing;
            height = height.max(y - 32.0 * spacing);
            width = width.max(nodes[i].rect.size.x);
        }
        let layer_width = local_x + width;
        if x > 0.0 && x + layer_width > target_width {
            x = 0.0;
            band_y += band_height + 64.0 * spacing;
            band_height = 0.0;
        }
        for (i, p) in local {
            result[i] = Point {
                x: x + p.x,
                y: band_y + p.y,
            };
        }
        x += layer_width + 48.0 * spacing;
        band_height = band_height.max(height);
    }
    result
}
/// Pack disconnected components instead of making a single, very tall layer.
fn place_graph(nodes: &[LayoutNode], edges: &[(usize, usize)], spacing: f32) -> Vec<Point> {
    let n = nodes.len();
    let mut seen = vec![false; n];
    let mut components = vec![];
    for root in 0..n {
        if seen[root] {
            continue;
        }
        let mut todo = vec![root];
        let mut component = vec![];
        seen[root] = true;
        while let Some(v) = todo.pop() {
            component.push(v);
            for &(a, b) in edges {
                let w = if a == v {
                    b
                } else if b == v {
                    a
                } else {
                    continue;
                };
                if !seen[w] {
                    seen[w] = true;
                    todo.push(w);
                }
            }
        }
        component.sort_unstable();
        components.push(component);
    }
    let area: f32 = nodes
        .iter()
        .map(|n| (n.rect.size.x + 48.0 * spacing) * (n.rect.size.y + 32.0 * spacing))
        .sum();
    let shelf_width = (area * 1.6).sqrt().max(1200.0);
    let mut result = vec![Point::default(); n];
    let (mut x, mut y, mut shelf_h) = (0.0, 0.0, 0.0_f32);
    for component in components {
        let map: BTreeMap<_, _> = component.iter().enumerate().map(|(i, &v)| (v, i)).collect();
        let local: Vec<_> = component.iter().map(|&i| nodes[i].clone()).collect();
        let links: Vec<_> = edges
            .iter()
            .filter_map(|(a, b)| Some((*map.get(a)?, *map.get(b)?)))
            .collect();
        let points = layered(&local, &links, spacing);
        let (width, height) = local
            .iter()
            .zip(&points)
            .fold((0.0_f32, 0.0_f32), |(w, h), (n, p)| {
                (w.max(p.x + n.rect.size.x), h.max(p.y + n.rect.size.y))
            });
        if x > 0.0 && x + width > shelf_width {
            x = 0.0;
            y += shelf_h + 64.0 * spacing;
            shelf_h = 0.0;
        }
        for (&i, p) in component.iter().zip(points) {
            result[i] = Point {
                x: p.x + x,
                y: p.y + y,
            };
        }
        x += width + 64.0 * spacing;
        shelf_h = shelf_h.max(height);
    }
    result
}

fn position_diagram(input: &LayoutInput, profile: LayoutProfile) -> LayoutInput {
    let spacing = profile.spacing();
    let mut next = input.clone();
    next.manual.retain(|id, _| {
        input.edges.iter().any(|e| {
            &e.id == id && input.fixed.contains(&e.source) && input.fixed.contains(&e.target)
        })
    });
    let movable: Vec<_> = input
        .nodes
        .iter()
        .filter(|n| !input.fixed.contains(&n.id))
        .cloned()
        .collect();
    let ids: BTreeMap<_, _> = movable
        .iter()
        .enumerate()
        .map(|(i, n)| (n.id.as_str(), i))
        .collect();
    let mut parent: Vec<_> = (0..movable.len()).collect();
    fn root(parent: &[usize], mut v: usize) -> usize {
        while parent[v] != v {
            v = parent[v];
        }
        v
    }
    for group in &input.groups {
        let members: Vec<_> = group
            .iter()
            .filter_map(|id| ids.get(id.as_str()).copied())
            .collect();
        if let Some(&first) = members.first() {
            for &v in &members {
                let (a, b) = (root(&parent, first), root(&parent, v));
                parent[b] = a;
            }
        }
    }
    let mut blocks: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for i in 0..movable.len() {
        blocks.entry(root(&parent, i)).or_default().push(i);
    }
    let mut macro_nodes = vec![];
    let mut local_positions = BTreeMap::new();
    let mut block_of = BTreeMap::new();
    for (block, members) in blocks.values().enumerate() {
        let local: Vec<_> = members.iter().map(|&i| movable[i].clone()).collect();
        let map: BTreeMap<_, _> = local
            .iter()
            .enumerate()
            .map(|(i, n)| (n.id.as_str(), i))
            .collect();
        let links: Vec<_> = input
            .edges
            .iter()
            .filter_map(|e| Some((*map.get(e.source.as_str())?, *map.get(e.target.as_str())?)))
            .collect();
        let points = place_graph(&local, &links, spacing);
        let padding = if members.len() > 1 {
            Point {
                x: 32.0 * spacing,
                y: 48.0 * spacing,
            }
        } else {
            Point::default()
        };
        let mut size = Point::default();
        for (node, p) in local.iter().zip(points) {
            let p = Point {
                x: p.x + padding.x,
                y: p.y + padding.y,
            };
            size.x = size.x.max(p.x + node.rect.size.x + padding.x);
            size.y = size.y.max(p.y + node.rect.size.y + 32.0 * spacing);
            local_positions.insert(node.id.clone(), p);
            block_of.insert(node.id.clone(), block);
        }
        macro_nodes.push(LayoutNode {
            id: local[0].id.clone(),
            rect: NodeRect {
                origin: Point::default(),
                size,
            },
        });
    }
    let links: Vec<_> = input
        .edges
        .iter()
        .filter_map(|e| {
            let (a, b) = (*block_of.get(&e.source)?, *block_of.get(&e.target)?);
            (a != b).then_some((a, b))
        })
        .collect();
    let mut block_positions = place_graph(&macro_nodes, &links, spacing);
    let mut occupied: Vec<_> = input
        .nodes
        .iter()
        .filter(|n| input.fixed.contains(&n.id))
        .map(|n| n.rect.expanded(16.0 * spacing))
        .collect();
    for (i, p) in block_positions.iter_mut().enumerate() {
        p.x += 48.0;
        p.y += 64.0;
        loop {
            let rect = NodeRect {
                origin: *p,
                size: macro_nodes[i].rect.size,
            };
            let overlaps: Vec<_> = occupied.iter().filter(|r| r.intersects(&rect)).collect();
            if overlaps.is_empty() {
                occupied.push(rect.expanded(16.0 * spacing));
                break;
            }
            p.y = overlaps
                .iter()
                .map(|r| r.bottom() + 32.0 * spacing)
                .fold(p.y, f32::max);
        }
    }
    for node in &mut next.nodes {
        if let Some(&block) = block_of.get(&node.id) {
            let p = local_positions[&node.id];
            node.rect.origin = Point {
                x: p.x + block_positions[block].x,
                y: p.y + block_positions[block].y,
            };
        }
    }
    next
}
pub fn place_diagram(input: &LayoutInput) -> BTreeMap<String, Point> {
    position_diagram(input, LayoutProfile::Compact)
        .nodes
        .into_iter()
        .map(|n| (n.id, n.rect.origin))
        .collect()
}
pub fn arrange_diagram(input: &LayoutInput) -> LayoutResult {
    arrange_diagram_with_profile(input, LayoutProfile::Compact)
}
pub fn arrange_diagram_with_profile(input: &LayoutInput, profile: LayoutProfile) -> LayoutResult {
    route_diagram(&position_diagram(input, profile))
}

// A shared occupancy grid makes obstacle checks O(1) during each orthogonal search.
struct RoutingGrid {
    origin: Point,
    step: f32,
    width: usize,
    height: usize,
    blocked: Vec<bool>,
    horizontal: Vec<u16>,
    vertical: Vec<u16>,
}
impl RoutingGrid {
    fn new(rects: &BTreeMap<String, NodeRect>) -> Self {
        let min_x = rects.values().map(|r| r.origin.x).fold(0.0, f32::min) - 96.0;
        let min_y = rects.values().map(|r| r.origin.y).fold(0.0, f32::min) - 96.0;
        let max_x = rects.values().map(|r| r.right()).fold(0.0, f32::max) + 96.0;
        let max_y = rects.values().map(|r| r.bottom()).fold(0.0, f32::max) + 96.0;
        let step = 16.0_f32
            .max((((max_x - min_x) * (max_y - min_y) / 250_000.0).sqrt() / 16.0).ceil() * 16.0);
        let origin = Point {
            x: (min_x / step).floor() * step,
            y: (min_y / step).floor() * step,
        };
        let width = ((max_x - origin.x) / step).ceil() as usize + 1;
        let height = ((max_y - origin.y) / step).ceil() as usize + 1;
        let mut grid = Self {
            origin,
            step,
            width,
            height,
            blocked: vec![false; width * height],
            horizontal: vec![0; width * height],
            vertical: vec![0; width * height],
        };
        for r in rects.values() {
            let r = r.expanded(12.0);
            let x0 = ((r.origin.x - origin.x) / step).ceil().max(0.0) as usize;
            let x1 = ((r.right() - origin.x) / step).floor() as usize;
            let y0 = ((r.origin.y - origin.y) / step).ceil().max(0.0) as usize;
            let y1 = ((r.bottom() - origin.y) / step).floor() as usize;
            for y in y0..=y1.min(height - 1) {
                for x in x0..=x1.min(width - 1) {
                    grid.blocked[y * width + x] = true;
                }
            }
        }
        grid
    }
    fn index(&self, p: Point) -> usize {
        let x = ((p.x - self.origin.x) / self.step)
            .round()
            .clamp(0.0, (self.width - 1) as f32) as usize;
        let y = ((p.y - self.origin.y) / self.step)
            .round()
            .clamp(0.0, (self.height - 1) as f32) as usize;
        y * self.width + x
    }
    fn point(&self, i: usize) -> Point {
        Point {
            x: self.origin.x + (i % self.width) as f32 * self.step,
            y: self.origin.y + (i / self.width) as f32 * self.step,
        }
    }
    fn path(&mut self, start: Point, end: Point) -> Vec<Point> {
        let (s, t) = (self.index(start), self.index(end));
        let n = self.width * self.height;
        let mut distance = vec![u32::MAX; n * 2];
        let mut previous = vec![usize::MAX; n * 2];
        let mut queue = BinaryHeap::new();
        let heuristic = |i: usize| {
            ((i % self.width).abs_diff(t % self.width) + (i / self.width).abs_diff(t / self.width))
                as u32
                * 10
        };
        for dir in 0..2 {
            distance[s * 2 + dir] = 0;
            queue.push(Reverse((heuristic(s), 0, s * 2 + dir)));
        }
        let mut finish = None;
        let mut visited = 0;
        while let Some(Reverse((_, cost, state))) = queue.pop() {
            if distance[state] != cost {
                continue;
            }
            let i = state / 2;
            let dir = state % 2;
            if i == t {
                finish = Some(state);
                break;
            }
            visited += 1;
            if visited > n * 2 {
                break;
            }
            let (x, y) = (i % self.width, i / self.width);
            for (valid, j, axis) in [
                (x > 0, i.saturating_sub(1), 0),
                (x + 1 < self.width, i + 1, 0),
                (y > 0, i.saturating_sub(self.width), 1),
                (y + 1 < self.height, i + self.width, 1),
            ] {
                if !valid || self.blocked[j] {
                    continue;
                }
                let overlap = if axis == 0 {
                    self.horizontal[i.min(j)]
                } else {
                    self.vertical[i.min(j)]
                };
                let crossing = if axis == 0 {
                    self.vertical[j]
                } else {
                    self.horizontal[j]
                };
                let next_cost = cost
                    + 10
                    + if dir != axis { 6 } else { 0 }
                    + u32::from(overlap.min(4)) * 80
                    + u32::from(crossing.min(4)) * 40;
                let next = j * 2 + axis;
                if next_cost < distance[next] {
                    distance[next] = next_cost;
                    previous[next] = state;
                    queue.push(Reverse((next_cost + heuristic(j), next_cost, next)));
                }
            }
        }
        let Some(mut state) = finish else {
            return vec![
                start,
                Point {
                    x: start.x,
                    y: self.origin.y,
                },
                Point {
                    x: end.x,
                    y: self.origin.y,
                },
                end,
            ];
        };
        let mut indices = vec![state / 2];
        while previous[state] != usize::MAX {
            state = previous[state];
            indices.push(state / 2);
        }
        indices.reverse();
        for pair in indices.windows(2) {
            let slot = pair[0].min(pair[1]);
            if pair[0] / self.width == pair[1] / self.width {
                self.horizontal[slot] = self.horizontal[slot].saturating_add(1);
            } else {
                self.vertical[slot] = self.vertical[slot].saturating_add(1);
            }
        }
        indices.into_iter().map(|i| self.point(i)).collect()
    }
}
pub fn simplify_path(points: Vec<Point>) -> Vec<Point> {
    let mut result: Vec<Point> = vec![];
    for p in points {
        if result.last() == Some(&p) {
            continue;
        }
        while result.len() > 1 {
            let (a, b) = (result[result.len() - 2], result[result.len() - 1]);
            if (a.x == b.x && b.x == p.x) || (a.y == b.y && b.y == p.y) {
                result.pop();
            } else {
                break;
            }
        }
        result.push(p);
    }
    result
}

pub fn route_diagram(input: &LayoutInput) -> LayoutResult {
    let rects: BTreeMap<_, _> = input.nodes.iter().map(|n| (n.id.clone(), n.rect)).collect();
    let obstacles: Vec<_> = rects.values().copied().collect();
    struct PortRequest {
        edge: String,
        source: bool,
        opposite_y: f32,
        preferred_y: f32,
    }
    let mut slots: BTreeMap<(String, bool), Vec<PortRequest>> = BTreeMap::new();
    let mut ports = BTreeMap::new();
    let mut sides = BTreeMap::new();
    for e in &input.edges {
        let (a, b) = (rects[&e.source], rects[&e.target]);
        let left = a.origin.x > b.origin.x;
        let right = left || e.source == e.target;
        let manual = input.manual.get(&e.id);
        let (left, right) = manual
            .map(|m| (m.source_left, m.target_right))
            .unwrap_or((left, right));
        let source_anchor = manual.and_then(|route| route.source_anchor);
        let target_anchor = manual.and_then(|route| route.target_anchor);
        sides.insert(
            e.id.clone(),
            (
                source_anchor.map(|anchor| anchor.side).unwrap_or(if left {
                    AnchorSide::Left
                } else {
                    AnchorSide::Right
                }),
                target_anchor.map(|anchor| anchor.side).unwrap_or(if right {
                    AnchorSide::Right
                } else {
                    AnchorSide::Left
                }),
            ),
        );
        if let Some(anchor) = source_anchor {
            ports.insert((e.id.clone(), true), resolve_table_anchor(a, anchor));
        } else {
            slots
                .entry((e.source.clone(), left))
                .or_default()
                .push(PortRequest {
                    edge: e.id.clone(),
                    source: true,
                    opposite_y: b.origin.y,
                    preferred_y: e.source_y,
                });
        }
        if let Some(anchor) = target_anchor {
            ports.insert((e.id.clone(), false), resolve_table_anchor(b, anchor));
        } else {
            slots
                .entry((e.target.clone(), !right))
                .or_default()
                .push(PortRequest {
                    edge: e.id.clone(),
                    source: false,
                    opposite_y: a.origin.y,
                    preferred_y: e.target_y,
                });
        }
    }
    for ((node, left), mut members) in slots {
        members.sort_by(|a, b| {
            a.preferred_y
                .total_cmp(&b.preferred_y)
                .then(a.opposite_y.total_cmp(&b.opposite_y))
                .then(a.edge.cmp(&b.edge))
                .then(a.source.cmp(&b.source))
        });
        let rect = rects[&node];
        let mut ys = vec![];
        let mut previous = 32.0;
        for request in &members {
            let y = request.preferred_y.max(previous + PORT_GAP);
            ys.push(y);
            previous = y;
        }
        let mut next = rect.size.y;
        for y in ys.iter_mut().rev() {
            *y = (*y).min(next - PORT_GAP);
            next = *y;
        }
        for (request, automatic_y) in members.into_iter().zip(ys) {
            let PortRequest { edge, source, .. } = request;
            let manual = input
                .manual
                .get(&edge)
                .and_then(|r| if source { r.source_y } else { r.target_y });
            let y = manual
                .unwrap_or(automatic_y)
                .clamp(12.0, rect.size.y - 12.0);
            ports.insert(
                (edge, source),
                Point {
                    x: if left { rect.origin.x } else { rect.right() },
                    y: rect.origin.y + y,
                },
            );
        }
    }
    let mut grid = RoutingGrid::new(&rects);
    let mut edges = BTreeMap::new();
    for e in &input.edges {
        let (start, end) = (ports[&(e.id.clone(), true)], ports[&(e.id.clone(), false)]);
        let (source_side, target_side) = sides[&e.id];
        let points = if let Some(manual) = input.manual.get(&e.id).filter(|r| !r.points.is_empty())
        {
            let mut p = vec![start];
            p.extend(&manual.points);
            p.push(end);
            if manual.source_anchor.is_some() || manual.target_anchor.is_some() {
                reconnect_route_endpoints(
                    &p,
                    (
                        rects[&e.source],
                        project_table_anchor(rects[&e.source], start, Some(source_side)),
                    ),
                    (
                        rects[&e.target],
                        project_table_anchor(rects[&e.target], end, Some(target_side)),
                    ),
                    &obstacles,
                )
            } else {
                // Legacy user-defined trunks retain their exact stored shape.
                p
            }
        } else if e.source == e.target && start == end && source_side == target_side {
            reconnect_route_endpoints(
                &[],
                (
                    rects[&e.source],
                    project_table_anchor(rects[&e.source], start, Some(source_side)),
                ),
                (
                    rects[&e.target],
                    project_table_anchor(rects[&e.target], end, Some(target_side)),
                ),
                &obstacles,
            )
        } else {
            let stub = |p: Point, side: AnchorSide| {
                let before = |value: f32, origin: f32| {
                    ((value - 24.0 - origin) / grid.step).floor() * grid.step + origin
                };
                let after = |value: f32, origin: f32| {
                    ((value + 24.0 - origin) / grid.step).ceil() * grid.step + origin
                };
                match side {
                    AnchorSide::Left => Point {
                        x: before(p.x, grid.origin.x),
                        y: p.y,
                    },
                    AnchorSide::Right => Point {
                        x: after(p.x, grid.origin.x),
                        y: p.y,
                    },
                    AnchorSide::Top => Point {
                        x: p.x,
                        y: before(p.y, grid.origin.y),
                    },
                    AnchorSide::Bottom => Point {
                        x: p.x,
                        y: after(p.y, grid.origin.y),
                    },
                }
            };
            let (a, b) = (stub(start, source_side), stub(end, target_side));
            let (ga, gb) = (grid.point(grid.index(a)), grid.point(grid.index(b)));
            let mut p = vec![start, a, ga];
            p.extend(grid.path(ga, gb));
            p.extend([gb, b, end]);
            let path = simplify_path(p);
            let edited_anchor = input.manual.get(&e.id).is_some_and(|route| {
                route.source_anchor.is_some() || route.target_anchor.is_some()
            });
            if edited_anchor && !clear_path(&path, &obstacles) {
                reconnect_route_endpoints(
                    &[],
                    (
                        rects[&e.source],
                        project_table_anchor(rects[&e.source], start, Some(source_side)),
                    ),
                    (
                        rects[&e.target],
                        project_table_anchor(rects[&e.target], end, Some(target_side)),
                    ),
                    &obstacles,
                )
            } else {
                path
            }
        };
        edges.insert(
            e.id.clone(),
            EdgeGeometry {
                points,
                bridges: vec![],
            },
        );
    }
    let ids: Vec<_> = edges.keys().cloned().collect();
    for (i, id) in ids.iter().enumerate() {
        let path = edges[id].points.clone();
        let mut bridges = vec![];
        for other in &ids[..i] {
            for a in path.windows(2) {
                for b in edges[other].points.windows(2) {
                    if let Some(p) = crossing(a[0], a[1], b[0], b[1])
                        && !bridges.contains(&p)
                    {
                        bridges.push(p);
                    }
                }
            }
        }
        edges.get_mut(id).expect("known edge").bridges = bridges;
    }
    LayoutResult { rects, edges }
}
fn crossing(a: Point, b: Point, c: Point, d: Point) -> Option<Point> {
    let (h0, h1, v0, v1) = if a.y == b.y && c.x == d.x {
        (a, b, c, d)
    } else if a.x == b.x && c.y == d.y {
        (c, d, a, b)
    } else {
        return None;
    };
    (v0.x > h0.x.min(h1.x)
        && v0.x < h0.x.max(h1.x)
        && h0.y > v0.y.min(v1.y)
        && h0.y < v0.y.max(v1.y))
    .then_some(Point { x: v0.x, y: h0.y })
}
