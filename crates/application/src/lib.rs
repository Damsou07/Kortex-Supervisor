//! Application ports and state machines independent of GUI and database drivers.
pub mod diagram;
use kortex_domain::*;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

pub trait CancelQuery: Send + Sync {
    fn cancel(&self);
}
pub trait DatabaseBackend: Send {
    fn catalog(&mut self) -> Result<Catalog>;
    fn page(&mut self, table: &Table, query: &PageQuery) -> Result<DataPage>;
    fn apply(&mut self, table: &Table, changes: &ChangeSet) -> Result<u64>;
    fn execute_sql(&mut self, sql: &str, limit: usize, emit: &mut dyn FnMut(SqlEvent));
    fn cancel_handle(&self) -> Arc<dyn CancelQuery>;
}
pub trait SecretStore {
    fn get(&self, profile: &str) -> Result<Option<String>>;
    fn set(&self, profile: &str, secret: &str) -> Result<()>;
    fn delete(&self, profile: &str) -> Result<()>;
}

#[derive(Default)]
pub struct RequestGate {
    next: u64,
    latest: BTreeMap<u64, u64>,
}
impl RequestGate {
    pub fn begin(&mut self, view: u64) -> u64 {
        self.next += 1;
        self.latest.insert(view, self.next);
        self.next
    }
    pub fn accepts(&self, view: u64, request: u64) -> bool {
        self.latest.get(&view) == Some(&request)
    }
    pub fn remove(&mut self, view: u64) {
        self.latest.remove(&view);
    }
}
#[derive(Default)]
pub struct GridState {
    pub page: DataPage,
    pub edits: BTreeMap<usize, BTreeMap<String, InputValue>>,
    pub deleted: BTreeSet<usize>,
    pub inserted: Vec<BTreeMap<String, InputValue>>,
    pub external_change: bool,
}
impl GridState {
    pub fn dirty(&self) -> bool {
        !self.edits.is_empty() || !self.deleted.is_empty() || !self.inserted.is_empty()
    }
    pub fn receive(&mut self, page: DataPage) {
        if self.dirty() {
            self.external_change |= self.page != page;
        } else {
            self.page = page;
            self.external_change = false;
        }
    }
    pub fn discard(&mut self) {
        self.edits.clear();
        self.deleted.clear();
        self.inserted.clear();
        self.external_change = false;
    }
    pub fn changes(&self) -> ChangeSet {
        let mut changes: Vec<_> = self
            .deleted
            .iter()
            .filter_map(|i| {
                self.page
                    .rows
                    .get(*i)
                    .cloned()
                    .map(|original| RowChange::Delete { original })
            })
            .collect();
        changes.extend(
            self.edits
                .iter()
                .filter(|(i, _)| !self.deleted.contains(i))
                .filter_map(|(i, values)| {
                    self.page
                        .rows
                        .get(*i)
                        .cloned()
                        .map(|original| RowChange::Update {
                            original,
                            values: values.clone(),
                        })
                }),
        );
        changes.extend(
            self.inserted
                .iter()
                .cloned()
                .map(|values| RowChange::Insert { values }),
        );
        ChangeSet { changes }
    }
}
