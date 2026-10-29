use kortex_application::diagram::{DiagramExecutor, LayoutRequest};
use kortex_domain::*;
use kortex_infrastructure::diagram_worker::DiagramWorker;
#[test]
fn cpu_worker_returns_the_revision_and_stops_when_dropped() {
    let mut worker = DiagramWorker::new();
    assert!(worker.submit(LayoutRequest {
        revision: 42,
        input: LayoutInput::default(),
        arrange: true
    }));
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    loop {
        if let Some(reply) = worker.try_receive() {
            assert_eq!(reply.revision, 42);
            assert!(reply.geometry.rects.is_empty());
            break;
        }
        assert!(std::time::Instant::now() < deadline);
        std::thread::yield_now();
    }
}
