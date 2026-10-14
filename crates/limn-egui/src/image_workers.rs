//! A small, nonblocking decode queue owned by one UI thread's raster cache.

use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};

const WORKERS: usize = 2;
const PENDING: usize = 32;
const PENDING_BYTES: usize = 16 * 1024 * 1024;

type Job = Box<dyn FnOnce() + Send>;

#[derive(Default)]
struct Queue {
    jobs: VecDeque<(usize, Job)>,
    bytes: usize,
    closed: bool,
}

#[derive(Default)]
struct Shared {
    queue: Mutex<Queue>,
    ready: Condvar,
}

pub(super) struct ImageWorkers(Arc<Shared>);

impl ImageWorkers {
    pub(super) fn new() -> std::io::Result<Self> {
        let workers = Self(Arc::new(Shared::default()));
        for index in 0..WORKERS {
            let shared = workers.0.clone();
            std::thread::Builder::new()
                .name(format!("limn-image-{index}"))
                .spawn(move || loop {
                    let job = {
                        let mut queue = shared.queue.lock().expect("image queue");
                        while queue.jobs.is_empty() && !queue.closed {
                            queue = shared.ready.wait(queue).expect("image queue");
                        }
                        if queue.closed {
                            return;
                        }
                        let (bytes, job) = queue.jobs.pop_front().expect("pending image");
                        queue.bytes -= bytes;
                        job
                    };
                    job();
                })?;
        }
        Ok(workers)
    }

    /// Refuse immediately when full; the caller retries on a later paint pass.
    pub(super) fn submit(&self, bytes: usize, job: impl FnOnce() + Send + 'static) -> bool {
        let mut queue = self.0.queue.lock().expect("image queue");
        if queue.jobs.len() >= PENDING || bytes > PENDING_BYTES.saturating_sub(queue.bytes) {
            return false;
        }
        queue.jobs.push_back((bytes, Box::new(job)));
        queue.bytes += bytes;
        self.0.ready.notify_one();
        true
    }

    pub(super) fn clear(&self) {
        let mut queue = self.0.queue.lock().expect("image queue");
        queue.jobs.clear();
        queue.bytes = 0;
    }
}

impl Drop for ImageWorkers {
    fn drop(&mut self) {
        let mut queue = self.0.queue.lock().expect("image queue");
        queue.closed = true;
        queue.jobs.clear();
        queue.bytes = 0;
        self.0.ready.notify_all();
        // Active decodes finish without blocking the UI thread. Their old
        // generation and receivers are gone, so they cannot publish a texture.
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::time::Duration;

    fn occupied() -> (ImageWorkers, Vec<mpsc::Sender<()>>) {
        let workers = ImageWorkers::new().unwrap();
        let (started, starts) = mpsc::channel();
        let mut release = Vec::new();
        for _ in 0..WORKERS {
            let (send, receive) = mpsc::channel();
            let started = started.clone();
            assert!(workers.submit(0, move || {
                started.send(()).unwrap();
                let _ = receive.recv();
            }));
            release.push(send);
        }
        for _ in 0..WORKERS {
            starts.recv_timeout(Duration::from_secs(2)).unwrap();
        }
        (workers, release)
    }

    #[test]
    fn running_jobs_and_pending_jobs_are_bounded_without_waiting_for_a_decode() {
        let (workers, release) = occupied();
        let (done, completed) = mpsc::channel();
        for _ in 0..PENDING {
            let done = done.clone();
            assert!(workers.submit(1, move || done.send(()).unwrap()));
        }
        assert!(!workers.submit(1, || panic!("queue overflow executed")));
        assert!(matches!(
            completed.try_recv(),
            Err(mpsc::TryRecvError::Empty)
        ));
        drop(release);
        for _ in 0..PENDING {
            completed.recv_timeout(Duration::from_secs(2)).unwrap();
        }
        let done = done.clone();
        assert!(workers.submit(1, move || done.send(()).unwrap()));
        completed.recv_timeout(Duration::from_secs(2)).unwrap();
    }

    #[test]
    fn pending_source_bytes_are_bounded_and_clear_drops_the_queue_immediately() {
        let (workers, _release) = occupied();
        let held = Arc::new(());
        let queued = held.clone();
        assert!(workers.submit(PENDING_BYTES, move || drop(queued)));
        assert_eq!(Arc::strong_count(&held), 2);
        assert!(!workers.submit(1, || panic!("byte overflow executed")));
        workers.clear();
        assert_eq!(Arc::strong_count(&held), 1);
        assert!(workers.submit(PENDING_BYTES, || {}));
    }

    #[test]
    fn the_largest_constructible_raster_source_fits_an_empty_queue() {
        use limn::{RasterImage, RasterImageFormat};

        let (workers, _release) = occupied();
        let max_bytes = 8 * 1024 * 1024_usize;
        let max_encoded = max_bytes.div_ceil(3) * 4 + 4;
        let source = RasterImage::new(
            "0".repeat(64),
            RasterImageFormat::Png,
            max_bytes as u32,
            Arc::from("A".repeat(max_encoded)),
            "Maximum synthetic source",
        )
        .expect("constructor accepts its documented maximum");
        assert!(workers.submit(source.data_base64().len(), move || drop(source)));
        assert!(
            RasterImage::new(
                "0".repeat(64),
                RasterImageFormat::Png,
                max_bytes as u32,
                Arc::from("A".repeat(max_encoded + 1)),
                "Oversized synthetic source",
            )
            .is_none(),
            "a permanently oversized job cannot reach the render queue"
        );
    }
}
