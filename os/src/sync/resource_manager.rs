//! Deadlock detection: separate banker's matrices for mutex and semaphore.

use alloc::vec;
use crate::sync::resource_check::ResourceCheck;
use crate::sync::{Mutex, Semaphore};
use crate::task::ProcessControlBlockInner;
use alloc::sync::Arc;

/// Resource deadlock detection manager using Banker's algorithm.
/// Maintains separate resource matrices for mutexes and semaphores.
pub struct ResourceManager {
    /// Deadlock detection matrix for mutexes.
    pub mutex_check: ResourceCheck,
    /// Deadlock detection matrix for semaphores.
    pub sem_check: ResourceCheck,
}

impl ResourceManager {
    /// Initialize resource manager from process inner state.
    /// Builds initial matrices based on existing mutexes and semaphores.
    pub fn from_process_inner(inner: &ProcessControlBlockInner) -> Self {
        let n = inner.tasks.len().max(1);

        let m_mutex = inner.mutex_list.len();
        let mut mutex_totals = vec![0usize; m_mutex];
        for j in 0..m_mutex {
            if inner.mutex_list[j].is_some() {
                mutex_totals[j] = 1;
            }
        }
        let mut mutex_check = ResourceCheck::new(n, mutex_totals);
        for j in 0..m_mutex {
            if mutex_check.total[j] == 0 {
                continue;
            }
            let mu = inner.mutex_list[j].as_ref().unwrap();
            let available_val = if mu.is_locked_for_deadlock() {
                0
            } else {
                1
            };
            mutex_check.set_available(j, available_val);
        }

        let m_sem = inner.semaphore_list.len();
        let mut sem_totals = vec![0usize; m_sem];
        for j in 0..m_sem {
            if let Some(sem) = inner.semaphore_list[j].as_ref() {
                sem_totals[j] = sem.total_count();
            }
        }
        let mut sem_check = ResourceCheck::new(n, sem_totals);
        for j in 0..m_sem {
            if sem_check.total[j] == 0 {
                continue;
            }
            let sem = inner.semaphore_list[j].as_ref().unwrap();
            let c = sem.current_count();
            let available_val = if c > 0 { c as usize } else { 0 };
            sem_check.set_available(j, available_val);
        }

        Self {
            mutex_check,
            sem_check,
        }
    }

    /// Ensure the resource matrices have at least `n` thread rows.
    pub fn ensure_threads(&mut self, n: usize) {
        self.mutex_check.ensure_threads(n);
        self.sem_check.ensure_threads(n);
    }

    /// Keep mutex matrix columns aligned with `mutex_list` indices (new slots / recycled holes).
    pub fn sync_mutex_list(&mut self, mutex_list: &[Option<Arc<dyn Mutex>>]) {
        while self.mutex_check.num_resources() < mutex_list.len() {
            self.mutex_check.push_resource(1);
        }
        for j in 0..mutex_list.len() {
            if mutex_list[j].is_some() && self.mutex_check.total[j] == 0 {
                self.mutex_check.total[j] = 1;
                self.mutex_check.set_available(j, 1);
            }
        }
    }

    /// Synchronize the semaphore matrix columns with the current semaphore list.
    pub fn sync_semaphore_list(&mut self, sem_list: &[Option<Arc<Semaphore>>]) {
        while self.sem_check.num_resources() < sem_list.len() {
            self.sem_check.push_resource(0);
        }
        for j in 0..sem_list.len() {
            if let Some(sem) = sem_list[j].as_ref() {
                if self.sem_check.total[j] == 0 {
                    let total = sem.total_count();
                    let c = sem.current_count();
                    self.sem_check.total[j] = total;
                    let available_val = if c > 0 { c as usize } else { 0 };
                    self.sem_check.set_available(j, available_val);
                }
            }
        }
    }
}
