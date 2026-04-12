/*
死锁检测（银行家算法 + 动态 Max）
*/
use alloc::vec;
use alloc::vec::Vec;

/// Result of a resource acquire attempt for deadlock avoidance.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AcquireResult {
    /// Shadow state already reflects the granted units (immediate grant).
    Granted,
    /// Safe to block; shadow reflects raised claim but not yet allocated.
    WillWait,
}

/// Resource matrix for deadlock detection using Banker's algorithm.
/// Tracks total resources, available resources, and allocation/max_claim/need for each thread.
pub struct ResourceCheck {
    /// Total instances per resource type (constant).
    pub total: Vec<usize>,
    available: Vec<usize>,
    allocation: Vec<Vec<usize>>,
    max_claim: Vec<Vec<usize>>,
    need: Vec<Vec<usize>>,
}

impl ResourceCheck {
    /// Create an empty resource check state (no threads, no resources).
    pub fn empty() -> Self {
        Self {
            total: Vec::new(),
            available: Vec::new(),
            allocation: Vec::new(),
            max_claim: Vec::new(),
            need: Vec::new(),
        }
    }

    /// `totals[j]` = total instances of resource j. Rows indexed by tid (0..num_threads).
    pub fn new(num_threads: usize, totals: Vec<usize>) -> Self {
        let m = totals.len();
        let available = totals.clone();
        Self {
            total: totals,
            available,
            allocation: vec![vec![0; m]; num_threads],
            max_claim: vec![vec![0; m]; num_threads],
            need: vec![vec![0; m]; num_threads],
        }
    }

    /// Get the number of threads in this resource matrix.
    pub fn num_threads(&self) -> usize {
        self.allocation.len()
    }

    /// Get the number of resource types tracked in this matrix.
    pub fn num_resources(&self) -> usize {
        self.total.len()
    }

    /// Ensure at least `n` thread rows exist.
    pub fn ensure_threads(&mut self, n: usize) {
        let m = self.num_resources();
        while self.allocation.len() < n {
            self.allocation.push(vec![0; m]);
            self.max_claim.push(vec![0; m]);
            self.need.push(vec![0; m]);
        }
    }

    /// Append a new resource column. `total_j` is its instance count.
    pub fn push_resource(&mut self, total_j: usize) {
        self.total.push(total_j);
        self.available.push(total_j);
        for i in 0..self.allocation.len() {
            self.allocation[i].push(0);
            self.max_claim[i].push(0);
            self.need[i].push(0);
        }
    }

    /// Set the available count for resource `j`.
    pub fn set_available(&mut self, j: usize, available: usize) {
        if j < self.available.len() {
            self.available[j] = available;
        }
    }

    fn recompute_need_row(&mut self, t: usize) {
        let m = self.num_resources();
        for j in 0..m {
            self.need[t][j] = self.max_claim[t][j].saturating_sub(self.allocation[t][j]);
        }
    }

    /// Banker's safety algorithm: true iff state is safe.
    pub fn is_safe(&self) -> bool {
        let m = self.num_resources();
        let n = self.num_threads();
        if m == 0 || n == 0 {
            return true;
        }
        let mut work = self.available.clone();
        let mut finish = vec![false; n];
        loop {
            let mut found = false;
            for i in 0..n {
                if finish[i] {
                    continue;
                }
                let ok = (0..m).all(|j| self.need[i][j] <= work[j]);
                if ok {
                    for j in 0..m {
                        work[j] += self.allocation[i][j];
                    }
                    finish[i] = true;
                    found = true;
                }
            }
            if !found {
                break;
            }
        }
        finish.iter().all(|&f| f)
    }

    /// Try to acquire `k` units of resource `j` for thread `t`.
    /// On success, either shadow already granted (`Granted`) or caller may block (`WillWait`).
    /// On failure, state is unchanged and the request would be unsafe (deadlock).
    pub fn try_acquire(&mut self, t: usize, j: usize, k: usize) -> Result<AcquireResult, ()> {
        if k == 0 {
            return Ok(AcquireResult::Granted);
        }
        if j >= self.total.len() || t >= self.allocation.len() {
            return Err(());
        }
        if self.allocation[t][j] + k > self.total[j] {
            return Err(());
        }

        let old_max = self.max_claim[t][j];
        let new_max = old_max.max(self.allocation[t][j] + k);
        self.max_claim[t][j] = new_max;
        self.recompute_need_row(t);

        if k <= self.available[j] {
            self.available[j] -= k;
            self.allocation[t][j] += k;
            self.recompute_need_row(t);
            if self.is_safe() {
                return Ok(AcquireResult::Granted);
            }
            // rollback grant + max bump
            self.allocation[t][j] -= k;
            self.available[j] += k;
            self.max_claim[t][j] = old_max;
            self.recompute_need_row(t);
            return Err(());
        }

        // Must wait: no grant yet
        if self.is_safe() {
            return Ok(AcquireResult::WillWait);
        }
        self.max_claim[t][j] = old_max;
        self.recompute_need_row(t);
        Err(())
    }

    /// After blocking wait, the kernel object has granted `k` units (pool was non-empty).
    pub fn complete_acquire_after_wait(&mut self, t: usize, j: usize, k: usize) {
        if k == 0 || j >= self.total.len() || t >= self.allocation.len() {
            return;
        }
        assert!(self.available[j] >= k);
        self.available[j] -= k;
        self.allocation[t][j] += k;
        self.recompute_need_row(t);
    }

    /// Thread `t` releases `k` units of resource `j` back to the pool.
    pub fn release(&mut self, t: usize, j: usize, k: usize) {
        if k == 0 || j >= self.total.len() || t >= self.allocation.len() {
            return;
        }
        self.allocation[t][j] -= k;
        self.available[j] += k;
        self.recompute_need_row(t);
    }

    /// Mutex unlock handoff: lock goes from `from` to `to` without a free interval.
    pub fn transfer_allocation(&mut self, j: usize, from: usize, to: usize, k: usize) {
        if k == 0 || j >= self.total.len() {
            return;
        }
        assert!(from < self.allocation.len() && to < self.allocation.len());
        assert!(self.allocation[from][j] >= k);
        self.allocation[from][j] -= k;
        self.allocation[to][j] += k;
        self.recompute_need_row(from);
        self.recompute_need_row(to);
    }

    /// Semaphore `up` wakes `waiter`: one instance goes directly to the waiter.
    pub fn semaphore_up_wake(&mut self, j: usize, waiter_tid: usize) {
        if j >= self.total.len() || waiter_tid >= self.allocation.len() {
            return;
        }
        self.allocation[waiter_tid][j] += 1;
        self.recompute_need_row(waiter_tid);
    }

    /// Semaphore `up` with no waiter: increment available pool.
    pub fn semaphore_up_no_waiter(&mut self, j: usize, k: usize) {
        if j >= self.total.len() {
            return;
        }
        self.available[j] += k;
    }

    /// Get the number of instances of resource `j` currently allocated to thread `t`.
    pub fn allocation(&self, t: usize, j: usize) -> usize {
        if t < self.allocation.len() && j < self.total.len() {
            self.allocation[t][j]
        } else {
            0
        }
    }
}
