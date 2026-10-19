//! Background work with answers: jobs go in, answers come out frames later. Thumbnails of
//! maps, prefabs and materials are made this way.
//!
//! On the desktop a few threads take the jobs. The web build has no threads, so there the
//! jobs wait in the queue and [`Pool::try_recv`] does some of them on the frame thread,
//! for a few milliseconds a frame, before it looks for answers. A job that asks for a
//! repaint when it is done keeps that going until the queue is empty.

use std::sync::mpsc;
use std::sync::Arc;

/// How long the web build works through the queue in one frame.
#[cfg(target_arch = "wasm32")]
const BUDGET: std::time::Duration = std::time::Duration::from_millis(8);

/// A queue of jobs of type `J`, each turned into an answer of type `R` by one function.
pub struct Pool<J, R> {
    jobs: mpsc::Sender<J>,
    answers: mpsc::Receiver<R>,
    /// The queue, the way back and the work, for the frame thread to run.
    #[cfg(target_arch = "wasm32")]
    inline: (mpsc::Receiver<J>, mpsc::Sender<R>, Arc<dyn Fn(J) -> R>),
    /// When the frame thread last stopped working through the queue: answers are taken
    /// in a loop, and the work is once a frame, not once a call.
    #[cfg(target_arch = "wasm32")]
    worked: std::cell::Cell<Option<web_time::Instant>>,
}

impl<J: Send + 'static, R: Send + 'static> Pool<J, R> {
    /// A pool doing `work` on `threads` threads named `name` (on the desktop).
    pub fn new(name: &str, threads: usize, work: impl Fn(J) -> R + Send + Sync + 'static) -> Self {
        let (jobs, queue) = mpsc::channel::<J>();
        let (done, answers) = mpsc::channel::<R>();
        let work = Arc::new(work);
        #[cfg(not(target_arch = "wasm32"))]
        {
            let queue = Arc::new(std::sync::Mutex::new(queue));
            for n in 0..threads.max(1) {
                let (queue, done, work) = (Arc::clone(&queue), done.clone(), Arc::clone(&work));
                let _ = std::thread::Builder::new().name(format!("{name} {n}")).spawn(move || loop {
                    let job = match queue.lock() {
                        Ok(queue) => queue.recv(),
                        Err(_) => return,
                    };
                    let Ok(job) = job else { return };
                    if done.send(work(job)).is_err() {
                        return;
                    }
                });
            }
            Pool { jobs, answers }
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = (name, threads);
            Pool { jobs, answers, inline: (queue, done, work), worked: std::cell::Cell::new(None) }
        }
    }

    /// Queues a job.
    pub fn send(&self, job: J) {
        let _ = self.jobs.send(job);
    }

    /// The next answer, if one is ready. On the web, does queued jobs first.
    pub fn try_recv(&self) -> Option<R> {
        #[cfg(target_arch = "wasm32")]
        {
            let (queue, done, work) = &self.inline;
            if self.worked.get().is_none_or(|at| at.elapsed() > BUDGET / 2) {
                let start = web_time::Instant::now();
                while start.elapsed() < BUDGET {
                    let Ok(job) = queue.try_recv() else { break };
                    let _ = done.send(work(job));
                }
                self.worked.set(Some(web_time::Instant::now()));
            }
        }
        self.answers.try_recv().ok()
    }
}
