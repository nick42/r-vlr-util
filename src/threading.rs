//! Thread pools and per-thread operation context.

use std::cell::RefCell;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, mpsc};
use std::thread::{self, JoinHandle};

type Job = Box<dyn FnOnce() + Send + 'static>;

enum Message {
    Run(Job),
    Stop,
}

/// A small fixed-size thread pool.
pub struct ThreadPool {
    sender: mpsc::Sender<Message>,
    workers: Vec<JoinHandle<()>>,
}

impl ThreadPool {
    #[must_use]
    pub fn new(size: usize) -> Self {
        assert!(size > 0, "thread pool size must be non-zero");
        let (sender, receiver) = mpsc::channel::<Message>();
        let receiver = Arc::new(Mutex::new(receiver));
        let workers = (0..size)
            .map(|_| {
                let receiver = Arc::clone(&receiver);
                thread::spawn(move || {
                    while let Ok(message) =
                        receiver.lock().expect("worker receiver poisoned").recv()
                    {
                        match message {
                            Message::Run(job) => job(),
                            Message::Stop => break,
                        }
                    }
                })
            })
            .collect();
        Self { sender, workers }
    }

    pub fn execute<T: Send + 'static>(
        &self,
        task: impl FnOnce() -> T + Send + 'static,
    ) -> TaskHandle<T> {
        let (sender, receiver) = mpsc::sync_channel(1);
        self.sender
            .send(Message::Run(Box::new(move || {
                let _ = sender.send(task());
            })))
            .expect("thread pool has shut down");
        TaskHandle { receiver }
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        for _ in &self.workers {
            let _ = self.sender.send(Message::Stop);
        }
        for worker in self.workers.drain(..) {
            let _ = worker.join();
        }
    }
}

pub struct TaskHandle<T> {
    receiver: mpsc::Receiver<T>,
}

impl<T> TaskHandle<T> {
    pub fn join(self) -> Result<T, mpsc::RecvError> {
        self.receiver.recv()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationContext {
    pub name: String,
    pub value: String,
}

#[derive(Clone, Debug)]
struct ContextEntry {
    id: u64,
    context: OperationContext,
}

thread_local! {
    static CONTEXTS: RefCell<Vec<ContextEntry>> = const { RefCell::new(Vec::new()) };
}

static NEXT_CONTEXT_ID: AtomicU64 = AtomicU64::new(1);

/// Adds context to the current thread until the returned guard is dropped.
#[must_use]
pub fn add_operation_context(name: impl Into<String>, value: impl Into<String>) -> ContextGuard {
    let id = NEXT_CONTEXT_ID.fetch_add(1, Ordering::Relaxed);
    CONTEXTS.with(|contexts| {
        contexts.borrow_mut().push(ContextEntry {
            id,
            context: OperationContext {
                name: name.into(),
                value: value.into(),
            },
        });
    });
    ContextGuard {
        id,
        thread_id: thread::current().id(),
    }
}

#[must_use]
pub fn current_operation_context() -> Vec<OperationContext> {
    CONTEXTS.with(|contexts| {
        contexts
            .borrow()
            .iter()
            .map(|entry| entry.context.clone())
            .collect()
    })
}

pub struct ContextGuard {
    id: u64,
    thread_id: thread::ThreadId,
}

impl Drop for ContextGuard {
    fn drop(&mut self) {
        assert_eq!(
            self.thread_id,
            thread::current().id(),
            "operation context guards must be dropped on their creating thread"
        );
        CONTEXTS.with(|contexts| contexts.borrow_mut().retain(|entry| entry.id != self.id));
    }
}

#[cfg(test)]
mod tests {
    use super::{ThreadPool, add_operation_context, current_operation_context};

    #[test]
    fn thread_pool_returns_results() {
        let pool = ThreadPool::new(2);
        let one = pool.execute(|| 20 + 22);
        let two = pool.execute(|| "result".to_owned());
        assert_eq!(one.join().unwrap(), 42);
        assert_eq!(two.join().unwrap(), "result");
    }

    #[test]
    fn operation_context_is_scoped_and_thread_local() {
        let outer = add_operation_context("request", "42");
        {
            let _inner = add_operation_context("operation", "read");
            assert_eq!(current_operation_context().len(), 2);
            let context_on_other_thread = std::thread::spawn(current_operation_context)
                .join()
                .unwrap();
            assert!(context_on_other_thread.is_empty());
        }
        assert_eq!(current_operation_context().len(), 1);
        drop(outer);
        assert!(current_operation_context().is_empty());
    }
}
