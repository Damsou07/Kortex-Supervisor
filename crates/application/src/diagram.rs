//! Diagram edits are local, transactional history entries; none can issue SQL.
use kortex_domain::*;

pub struct LayoutRequest {
    pub revision: u64,
    pub input: LayoutInput,
    pub arrange: bool,
}
pub struct LayoutReply {
    pub revision: u64,
    pub geometry: LayoutResult,
}
pub trait DiagramExecutor {
    fn submit(&mut self, request: LayoutRequest) -> bool;
    fn try_receive(&mut self) -> Option<LayoutReply>;
}
#[derive(Clone, Debug, Default)]
pub struct GroupDraft {
    pub id: Option<u64>,
    pub name: String,
    pub members: Vec<String>,
    pub color: [u8; 4],
}
impl GroupDraft {
    pub fn valid(&self, catalog: &Catalog) -> bool {
        !self.name.trim().is_empty()
            && self
                .members
                .iter()
                .any(|id| catalog.tables.iter().any(|t| &t.stable_id == id))
    }
    pub fn save(
        &self,
        catalog: &Catalog,
        diagram: &mut Diagram,
        history: &mut LayoutHistory,
    ) -> bool {
        if !self.valid(catalog) {
            return false;
        }
        let mut members = self.members.clone();
        members.retain(|id| catalog.tables.iter().any(|t| &t.stable_id == id));
        members.sort();
        members.dedup();
        let id = self
            .id
            .unwrap_or_else(|| diagram.groups.iter().map(|g| g.id).max().unwrap_or(0) + 1);
        let mut next = diagram.clone();
        if let Some(g) = next.groups.iter_mut().find(|g| g.id == id) {
            g.name = self.name.trim().into();
            g.color = self.color;
            g.members = members;
        } else {
            next.groups.push(Group {
                id,
                name: self.name.trim().into(),
                color: self.color,
                members,
                origin: Point::default(),
                size: Point::default(),
            });
        }
        reframe(catalog, &mut next, Some(id));
        if next != *diagram {
            history.record(diagram);
            *diagram = next;
        }
        true
    }
}
pub fn remove_group(id: u64, diagram: &mut Diagram, history: &mut LayoutHistory) {
    if diagram.groups.iter().any(|g| g.id == id) {
        history.record(diagram);
        diagram.groups.retain(|g| g.id != id);
    }
}
pub fn reframe_groups(catalog: &Catalog, diagram: &mut Diagram) {
    reframe(catalog, diagram, None);
}
fn reframe(catalog: &Catalog, diagram: &mut Diagram, only: Option<u64>) {
    let input = LayoutInput::new(catalog, diagram);
    for group in &mut diagram.groups {
        if only.is_some_and(|id| group.id != id) {
            continue;
        }
        let members: Vec<_> = input
            .nodes
            .iter()
            .filter(|n| group.members.contains(&n.id))
            .collect();
        if members.is_empty() {
            continue;
        }
        let min_x = members
            .iter()
            .map(|n| n.rect.origin.x)
            .fold(f32::INFINITY, f32::min)
            - 24.0;
        let min_y = members
            .iter()
            .map(|n| n.rect.origin.y)
            .fold(f32::INFINITY, f32::min)
            - 44.0;
        let max_x = members
            .iter()
            .map(|n| n.rect.right())
            .fold(f32::NEG_INFINITY, f32::max)
            + 24.0;
        let max_y = members
            .iter()
            .map(|n| n.rect.bottom())
            .fold(f32::NEG_INFINITY, f32::max)
            + 24.0;
        group.origin = Point { x: min_x, y: min_y };
        group.size = Point {
            x: max_x - min_x,
            y: max_y - min_y,
        };
    }
}
pub fn apply_arrangement(
    catalog: &Catalog,
    diagram: &mut Diagram,
    history: &mut LayoutHistory,
    geometry: &LayoutResult,
    scope: Option<&[String]>,
) {
    history.record(diagram);
    for (id, rect) in &geometry.rects {
        if scope.is_none_or(|s| s.contains(id)) {
            diagram.positions.insert(id.clone(), rect.origin);
        }
    }
    for table in &catalog.tables {
        for fk in &table.foreign_keys {
            let target = catalog.tables.iter().find(|t| t.id == fk.target);
            if scope.is_none_or(|s| {
                s.contains(&table.stable_id) || target.is_some_and(|t| s.contains(&t.stable_id))
            }) {
                diagram.routes.remove(&fk.id);
            }
        }
    }
    if let Some(scope) = scope {
        let groups: Vec<_> = diagram
            .groups
            .iter()
            .filter(|g| g.members.iter().any(|id| scope.contains(id)))
            .map(|g| g.id)
            .collect();
        for id in groups {
            reframe(catalog, diagram, Some(id));
        }
    } else {
        reframe_groups(catalog, diagram);
    }
}
