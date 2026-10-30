use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, VecDeque};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cardinality {
    OptionalOne,
    One,
    Many,
}
impl Cardinality {
    pub fn label(self) -> &'static str {
        match self {
            Self::OptionalOne => "0..1",
            Self::One => "1..1",
            Self::Many => "0..N",
        }
    }
}
pub fn cardinalities(table: &Table, fk: &ForeignKey) -> (Cardinality, Cardinality) {
    let required: Vec<_> = fk
        .columns
        .iter()
        .map(|name| table.columns.iter().any(|c| c.name == *name && !c.nullable))
        .collect();
    let mandatory = !required.is_empty()
        && if fk.match_full {
            required.iter().any(|v| *v)
        } else {
            required.iter().all(|v| *v)
        };
    let unique = std::iter::once(&table.primary_key)
        .chain(table.unique_keys.iter())
        .any(|key| !key.is_empty() && key.iter().all(|c| fk.columns.contains(c)));
    (
        if mandatory {
            Cardinality::One
        } else {
            Cardinality::OptionalOne
        },
        if unique {
            Cardinality::OptionalOne
        } else {
            Cardinality::Many
        },
    )
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AnchorSide {
    Left,
    Right,
    Top,
    Bottom,
}
impl AnchorSide {
    /// Unit vector pointing away from the table.
    pub fn normal(self) -> Point {
        match self {
            Self::Left => Point { x: -1.0, y: 0.0 },
            Self::Right => Point { x: 1.0, y: 0.0 },
            Self::Top => Point { x: 0.0, y: -1.0 },
            Self::Bottom => Point { x: 0.0, y: 1.0 },
        }
    }
}
/// Distance along a table side in diagram units, independent of pan and zoom.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct TableAnchor {
    pub side: AnchorSide,
    pub offset: f32,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Group {
    pub id: u64,
    pub name: String,
    pub origin: Point,
    pub size: Point,
    pub members: Vec<String>,
    pub color: [u8; 4],
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Route {
    pub points: Vec<Point>,
    pub source_left: bool,
    pub target_right: bool,
    #[serde(default)]
    pub source_y: Option<f32>,
    #[serde(default)]
    pub target_y: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_anchor: Option<TableAnchor>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_anchor: Option<TableAnchor>,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Diagram {
    pub positions: BTreeMap<String, Point>,
    pub routes: BTreeMap<String, Route>,
    pub groups: Vec<Group>,
}
impl Diagram {
    pub fn reconcile(&mut self, tables: &[Table]) {
        let catalog = Catalog {
            tables: tables.to_vec(),
            ..Catalog::default()
        };
        let mut input = LayoutInput::new(&catalog, self);
        input.fixed = self.positions.keys().cloned().collect();
        if tables
            .iter()
            .all(|t| self.positions.contains_key(&t.stable_id))
        {
            return;
        }
        let next = place_diagram(&input);
        for t in tables {
            self.positions
                .entry(t.stable_id.clone())
                .or_insert(next[&t.stable_id]);
        }
    }
    pub fn move_tables(&mut self, ids: &[String], delta: Point) {
        for id in ids {
            if let Some(p) = self.positions.get_mut(id) {
                p.x += delta.x;
                p.y += delta.y;
            }
        }
    }
    pub fn move_group(&mut self, id: u64, delta: Point) {
        if let Some(group) = self.groups.iter_mut().find(|g| g.id == id) {
            group.origin.x += delta.x;
            group.origin.y += delta.y;
            let members = group.members.clone();
            self.move_tables(&members, delta);
        }
    }
}
#[derive(Default)]
pub struct LayoutHistory {
    past: VecDeque<Diagram>,
    future: Vec<Diagram>,
}
impl LayoutHistory {
    pub fn record(&mut self, diagram: &Diagram) {
        if self.past.back() != Some(diagram) {
            self.past.push_back(diagram.clone());
        }
        if self.past.len() > 100 {
            self.past.pop_front();
        }
        self.future.clear();
    }
    pub fn undo(&mut self, diagram: &mut Diagram) -> bool {
        if let Some(previous) = self.past.pop_back() {
            self.future.push(diagram.clone());
            *diagram = previous;
            true
        } else {
            false
        }
    }
    pub fn redo(&mut self, diagram: &mut Diagram) -> bool {
        if let Some(next) = self.future.pop() {
            self.past.push_back(diagram.clone());
            *diagram = next;
            true
        } else {
            false
        }
    }
}
