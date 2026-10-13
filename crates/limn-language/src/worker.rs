use crate::{AnalysisSnapshot, Cancellation, Language, ParserEngine};
use limn_editor::BufferSnapshot;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    mpsc, Arc, Mutex, OnceLock, Weak,
};

type Job = (BufferSnapshot, Language, u64);
struct Slot {
    job: Option<Job>,
    queued: bool,
    result: Option<Result<AnalysisSnapshot, String>>,
}
struct State {
    slot: Mutex<Slot>,
    generation: Arc<AtomicU64>,
}
/// Lazily shared two-thread parser service. Each client retains just the latest input and
/// latest output. Superseding a job interrupts parsing/query traversal of the old snapshot.
pub struct AnalysisWorker {
    state: Arc<State>,
}
impl Default for AnalysisWorker {
    fn default() -> Self {
        Self::new()
    }
}
impl AnalysisWorker {
    /// A client of the shared parser threads, which start on the first `submit` of any client.
    pub fn new() -> Self {
        Self {
            state: Arc::new(State {
                slot: Mutex::new(Slot {
                    job: None,
                    queued: false,
                    result: None,
                }),
                generation: Arc::new(AtomicU64::new(0)),
            }),
        }
    }
    /// False means the bounded shared queue is full. Caller may retry on the next frame.
    pub fn submit(&self, snapshot: BufferSnapshot, language: Language) -> bool {
        let Ok(mut slot) = self.state.slot.lock() else {
            return false;
        };
        let generation = self
            .state
            .generation
            .fetch_add(1, Ordering::AcqRel)
            .wrapping_add(1);
        slot.job = Some((snapshot, language, generation));
        slot.result = None;
        if slot.queued {
            return true;
        }
        match pool().try_send(Arc::downgrade(&self.state)) {
            Ok(()) => {
                slot.queued = true;
                true
            }
            Err(_) => {
                slot.job = None;
                false
            }
        }
    }
    /// Take the result for the latest submitted snapshot once it is ready. Each result is
    /// returned once.
    pub fn try_recv(&self) -> Option<Result<AnalysisSnapshot, String>> {
        self.state.slot.lock().ok()?.result.take()
    }
    /// Drop the waiting job and any unread result, and interrupt an analysis in progress.
    pub fn cancel(&self) {
        self.state.generation.fetch_add(1, Ordering::Release);
        if let Ok(mut slot) = self.state.slot.lock() {
            slot.job = None;
            slot.result = None;
        }
    }
}
impl Drop for AnalysisWorker {
    fn drop(&mut self) {
        self.cancel();
    }
}
fn pool() -> &'static mpsc::SyncSender<Weak<State>> {
    static POOL: OnceLock<mpsc::SyncSender<Weak<State>>> = OnceLock::new();
    POOL.get_or_init(|| {
        let (sender, receiver) = mpsc::sync_channel::<Weak<State>>(64);
        let receiver = Arc::new(Mutex::new(receiver));
        for index in 0..2 {
            let receiver = receiver.clone();
            std::thread::Builder::new()
                .name(format!("language-parse-{index}"))
                .spawn(move || {
                    let mut parser = ParserEngine::default();
                    loop {
                        let state = match receiver
                            .lock()
                            .ok()
                            .and_then(|receiver| receiver.recv().ok())
                            .and_then(|weak| weak.upgrade())
                        {
                            Some(state) => state,
                            None => continue,
                        };
                        let job = state.slot.lock().ok().and_then(|mut slot| {
                            slot.queued = false;
                            slot.job.take()
                        });
                        let Some((snapshot, language, generation)) = job else {
                            continue;
                        };
                        let cancellation =
                            Cancellation::for_generation(state.generation.clone(), generation);
                        let result = parser.analyze(&snapshot, language, &cancellation);
                        if !cancellation.is_cancelled() {
                            if let Ok(mut slot) = state.slot.lock() {
                                if !cancellation.is_cancelled() {
                                    slot.result = Some(result);
                                }
                            }
                        }
                    }
                })
                .expect("language worker thread creation failed");
        }
        sender
    })
}
