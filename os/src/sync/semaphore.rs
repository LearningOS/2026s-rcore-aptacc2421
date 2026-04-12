//! Semaphore

use crate::sync::UPSafeCell;
use crate::task::{block_current_and_run_next, current_task, wakeup_task, TaskControlBlock};
use alloc::{collections::VecDeque, sync::Arc};

/// semaphore structure
pub struct Semaphore {
    /// semaphore inner
    pub inner: UPSafeCell<SemaphoreInner>,
}

pub struct SemaphoreInner {
    pub count: isize,
    /// Initial resource count (constant, for deadlock detection).
    pub total: usize,
    pub wait_queue: VecDeque<Arc<TaskControlBlock>>,
}

impl Semaphore {
    /// Tid at the front of the wait queue (for deadlock shadow updates), if any.
    pub fn peek_waiter_tid(&self) -> Option<usize> {
        let inner = self.inner.exclusive_access();
        inner.wait_queue.front().map(|task| {
            let task_inner = task.inner_exclusive_access();
            task_inner.res.as_ref().unwrap().tid
        })
    }

    /// Current counter value (for initializing deadlock state).
    pub fn current_count(&self) -> isize {
        self.inner.exclusive_access().count
    }

    /// Get the total count (capacity) of this semaphore.
    pub fn total_count(&self) -> usize {
        self.inner.exclusive_access().total
    }

    /// Create a new semaphore
    pub fn new(res_count: usize) -> Self {
        trace!("kernel: Semaphore::new");
        Self {
            inner: unsafe {
                UPSafeCell::new(SemaphoreInner {
                    count: res_count as isize,
                    total: res_count,
                    wait_queue: VecDeque::new(),
                })
            },
        }
    }

    /// up operation of semaphore
    pub fn up(&self) {
        trace!("kernel: Semaphore::up");
        let mut inner = self.inner.exclusive_access();
        inner.count += 1;
        if inner.count <= 0 {
            if let Some(task) = inner.wait_queue.pop_front() {
                wakeup_task(task);
            }
        }
    }

    /// down operation of semaphore
    pub fn down(&self) {
        trace!("kernel: Semaphore::down");
        let mut inner = self.inner.exclusive_access();
        inner.count -= 1;
        if inner.count < 0 {
            inner.wait_queue.push_back(current_task().unwrap());
            drop(inner);
            block_current_and_run_next();
        }
    }
}
