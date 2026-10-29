//! One queued CPU job and one latest reply. Dropping the handle shuts the worker down.
use kortex_application::diagram::{DiagramExecutor, LayoutReply, LayoutRequest};
use std::sync::{Arc, Mutex, mpsc};

pub struct DiagramWorker {
    sender: mpsc::SyncSender<LayoutRequest>,
    reply: Arc<Mutex<Option<LayoutReply>>>,
}
impl Default for DiagramWorker {
    fn default() -> Self {
        Self::new()
    }
}
impl DiagramWorker {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::sync_channel::<LayoutRequest>(1);
        let reply = Arc::new(Mutex::new(None));
        let output = reply.clone();
        std::thread::spawn(move || {
            while let Ok(mut request) = receiver.recv() {
                while let Ok(next) = receiver.try_recv() {
                    request = next;
                }
                let geometry = if request.arrange {
                    kortex_domain::arrange_diagram(&request.input)
                } else {
                    kortex_domain::route_diagram(&request.input)
                };
                if let Ok(mut slot) = output.lock() {
                    *slot = Some(LayoutReply {
                        revision: request.revision,
                        geometry,
                    });
                }
            }
        });
        Self { sender, reply }
    }
}
impl DiagramExecutor for DiagramWorker {
    fn submit(&mut self, request: LayoutRequest) -> bool {
        self.sender.try_send(request).is_ok()
    }
    fn try_receive(&mut self) -> Option<LayoutReply> {
        self.reply.lock().ok()?.take()
    }
}
