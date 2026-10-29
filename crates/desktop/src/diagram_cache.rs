//! Geometry cache owned by a view. World geometry never depends on pan or zoom.
use kortex_application::diagram::{self, DiagramExecutor, LayoutRequest};
use kortex_domain::*;
use kortex_infrastructure::diagram_worker::DiagramWorker;

struct Pending {
    revision: u64,
    source: LayoutInput,
    scope: Option<Vec<String>>,
    arrange: bool,
}

#[derive(Default)]
pub struct DiagramCache {
    pub geometry: LayoutResult,
    input: Option<LayoutInput>,
    worker: Option<DiagramWorker>,
    pending: Option<Pending>,
    revision: u64,
    pub arrange: Option<(Option<Vec<String>>, LayoutProfile)>,
}
impl DiagramCache {
    pub fn prepare(&mut self, catalog: &Catalog, layout: &Diagram) {
        let input = LayoutInput::new(catalog, layout);
        self.geometry = route_diagram(&input);
        self.input = Some(input);
    }
    pub fn busy(&self) -> bool {
        self.pending.is_some() || self.arrange.is_some()
    }
    pub fn update(
        &mut self,
        catalog: &Catalog,
        layout: &mut Diagram,
        history: &mut LayoutHistory,
        dragging: bool,
    ) -> bool {
        let mut changed = false;
        let current = LayoutInput::new(catalog, layout);
        if let Some(reply) = self.worker.as_mut().and_then(|w| w.try_receive())
            && let Some(pending) = self.pending.take()
            && pending.revision == reply.revision
            && pending.source == current
        {
            if pending.arrange {
                diagram::apply_arrangement(
                    catalog,
                    layout,
                    history,
                    &reply.geometry,
                    pending.scope.as_deref(),
                );
                changed = true;
            }
            self.geometry = reply.geometry;
            self.input = Some(LayoutInput::new(catalog, layout));
        }
        let current = LayoutInput::new(catalog, layout);
        if !dragging
            && self.pending.is_none()
            && (self.arrange.is_some() || self.input.as_ref() != Some(&current))
        {
            let mut input = LayoutInput::new(catalog, layout);
            let request = self.arrange.clone();
            let scope = request.as_ref().and_then(|(scope, _)| scope.clone());
            if let Some(scope) = &scope {
                input.fixed = input
                    .nodes
                    .iter()
                    .filter(|n| !scope.contains(&n.id))
                    .map(|n| n.id.clone())
                    .collect();
            }
            self.revision += 1;
            if self
                .worker
                .get_or_insert_with(DiagramWorker::new)
                .submit(LayoutRequest {
                    revision: self.revision,
                    input,
                    arrange: request.as_ref().map(|(_, profile)| *profile),
                })
            {
                self.pending = Some(Pending {
                    revision: self.revision,
                    source: LayoutInput::new(catalog, layout),
                    scope,
                    arrange: request.is_some(),
                });
                self.arrange = None;
            }
        }
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn obsolete_arrangement_cannot_overwrite_a_move_and_unchanged_input_is_cached() {
        let catalog = Catalog {
            tables: vec![Table {
                stable_id: "t".into(),
                id: ObjectId::new("main", "t"),
                ..Table::default()
            }],
            ..Catalog::default()
        };
        let mut d = Diagram::default();
        d.reconcile(&catalog.tables);
        let mut cache = DiagramCache::default();
        cache.prepare(&catalog, &d);
        let mut h = LayoutHistory::default();
        cache.update(&catalog, &mut d, &mut h, false);
        assert_eq!(cache.revision, 0);
        cache.arrange = Some((None, LayoutProfile::Compact));
        cache.update(&catalog, &mut d, &mut h, false);
        d.positions.get_mut("t").unwrap().x += 1000.0;
        let moved = d.positions.clone();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        while cache.busy() {
            cache.update(&catalog, &mut d, &mut h, false);
            assert!(std::time::Instant::now() < deadline);
            std::thread::yield_now();
        }
        assert_eq!(d.positions, moved);
        assert!(!h.undo(&mut d));
        let revision = cache.revision;
        cache.update(&catalog, &mut d, &mut h, false);
        assert_eq!(cache.revision, revision);
    }
}
