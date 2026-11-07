#![deny(unsafe_code)]

use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::config::default_pool_threads;
use crate::tracker::ThreadStatus;

#[derive(Debug, Clone)]
pub struct CompilationThreadPool {
    threads: usize,
}

impl CompilationThreadPool {
    pub fn new(threads: usize) -> Self {
        Self {
            threads: threads.max(1),
        }
    }

    pub fn default_pool() -> Self {
        Self::new(default_pool_threads())
    }

    pub fn threads(&self) -> usize {
        self.threads
    }

    pub fn run_parallel<T, R, F>(&self, tasks: Vec<T>, f: F) -> Result<Vec<R>, String>
    where
        T: Send,
        R: Send,
        F: Fn(usize, T) -> Result<R, String> + Sync + Send,
    {
        self.run_with_tracker(tasks, f, |_, _| {})
    }

    pub fn run_with_tracker<T, R, F, P>(
        &self,
        tasks: Vec<T>,
        f: F,
        on_progress: P,
    ) -> Result<Vec<R>, String>
    where
        T: Send,
        R: Send,
        F: Fn(usize, T) -> Result<R, String> + Sync + Send,
        P: Fn(usize, ThreadStatus) + Sync + Send,
    {
        if tasks.is_empty() {
            return Ok(Vec::new());
        }

        let total = tasks.len();
        let worker_count = self.threads.min(total).max(1);
        let task_idx = AtomicUsize::new(0);
        let err_slot = Mutex::new(None);

        let task_slots: Vec<Mutex<Option<T>>> =
            tasks.into_iter().map(|t| Mutex::new(Some(t))).collect();
        let results: Vec<Mutex<Option<R>>> = (0..total).map(|_| Mutex::new(None)).collect();

        Self::spawn_workers(
            worker_count,
            total,
            &task_idx,
            &task_slots,
            &results,
            &err_slot,
            &f,
            &on_progress,
        )?;

        if let Some(err) = err_slot
            .into_inner()
            .map_err(|_| "mutex poisoned".to_string())?
        {
            return Err(err);
        }

        Self::collect_results(results)
    }

    #[allow(clippy::too_many_arguments)]
    fn spawn_workers<T, R, F, P>(
        worker_count: usize,
        total: usize,
        task_idx: &AtomicUsize,
        task_slots: &[Mutex<Option<T>>],
        results: &[Mutex<Option<R>>],
        err_slot: &Mutex<Option<String>>,
        f: &F,
        on_progress: &P,
    ) -> Result<(), String>
    where
        T: Send,
        R: Send,
        F: Fn(usize, T) -> Result<R, String> + Sync + Send,
        P: Fn(usize, ThreadStatus) + Sync + Send,
    {
        std::thread::scope(|s| {
            let mut handles = Vec::with_capacity(worker_count);
            for worker_i in 0..worker_count {
                let thread_id = worker_i + 1;
                handles.push(s.spawn(move || {
                    Self::worker_loop(
                        thread_id,
                        total,
                        task_idx,
                        task_slots,
                        results,
                        err_slot,
                        f,
                        on_progress,
                    );
                }));
            }

            for handle in handles {
                handle
                    .join()
                    .map_err(|_| "worker thread panicked".to_string())?;
            }
            Ok(())
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn worker_loop<T, R, F, P>(
        thread_id: usize,
        total: usize,
        task_idx: &AtomicUsize,
        task_slots: &[Mutex<Option<T>>],
        results: &[Mutex<Option<R>>],
        err_slot: &Mutex<Option<String>>,
        f: &F,
        on_progress: &P,
    ) where
        T: Send,
        R: Send,
        F: Fn(usize, T) -> Result<R, String> + Sync + Send,
        P: Fn(usize, ThreadStatus) + Sync + Send,
    {
        loop {
            if err_slot.lock().is_ok_and(|guard| guard.is_some()) {
                break;
            }

            let i = task_idx.fetch_add(1, Ordering::SeqCst);
            if i >= total {
                break;
            }

            let task = match task_slots[i].lock() {
                Ok(mut guard) => guard.take(),
                Err(_) => None,
            };

            let Some(task_item) = task else {
                continue;
            };

            match f(thread_id, task_item) {
                Ok(res) => {
                    if let Ok(mut guard) = results[i].lock() {
                        *guard = Some(res);
                    }
                }
                Err(e) => {
                    if let Ok(mut guard) = err_slot.lock()
                        && guard.is_none()
                    {
                        *guard = Some(e);
                    }
                    break;
                }
            }
        }

        on_progress(thread_id, ThreadStatus::Idle);
    }

    fn collect_results<R>(results: Vec<Mutex<Option<R>>>) -> Result<Vec<R>, String> {
        let mut output = Vec::with_capacity(results.len());
        for slot in results {
            let val = slot
                .into_inner()
                .map_err(|_| "mutex poisoned".to_string())?
                .ok_or_else(|| "task result slot empty".to_string())?;
            output.push(val);
        }
        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compilation_thread_pool_execution() {
        let pool = CompilationThreadPool::new(4);
        assert_eq!(pool.threads(), 4);

        let tasks = vec![1, 2, 3, 4, 5];
        let results = pool
            .run_parallel(tasks, |_thread_id, x| Ok(x * 10))
            .expect("run tasks");
        assert_eq!(results, vec![10, 20, 30, 40, 50]);
    }

    #[test]
    fn test_compilation_thread_pool_empty() {
        let pool = CompilationThreadPool::new(2);
        let tasks: Vec<i32> = Vec::new();
        let results = pool.run_parallel(tasks, |_t, x| Ok(x)).expect("empty run");
        assert!(results.is_empty());
    }

    #[test]
    fn test_compilation_thread_pool_error_handling() {
        let pool = CompilationThreadPool::new(2);
        let tasks = vec![1, 2, 3];
        let res = pool.run_parallel(tasks, |_t, x| {
            if x == 2 {
                Err("failed task 2".to_string())
            } else {
                Ok(x)
            }
        });
        assert!(res.is_err());
        assert_eq!(res.unwrap_err(), "failed task 2");
    }
}
