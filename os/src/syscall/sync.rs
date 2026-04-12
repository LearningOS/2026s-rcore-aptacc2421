use crate::sync::{
    AcquireResult, Condvar, Mutex, MutexBlocking, MutexSpin, ResourceManager, Semaphore,
};
use crate::task::{block_current_and_run_next, current_process, current_task};
use crate::timer::{add_timer, get_time_ms};
use alloc::sync::Arc;

/// User-visible error when the banker detects an unsafe (deadlock) state.
pub const SYSCALL_DEADLOCK: isize = -0xDEAD;

fn current_tid() -> usize {
    current_task()
        .unwrap()
        .inner_exclusive_access()
        .res
        .as_ref()
        .unwrap()
        .tid
}

/// sleep syscall
pub fn sys_sleep(ms: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_sleep",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    //println!("sys_sleep for {} ms", ms);
    let expire_ms = get_time_ms() + ms;
    let task = current_task().unwrap();
    add_timer(expire_ms, task);
    //println!("block current thread and run next");
    block_current_and_run_next();
    //println!("sys_sleep for {} ms done", ms);
    0
}
/// mutex create syscall
pub fn sys_mutex_create(blocking: bool) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mutex: Option<Arc<dyn Mutex>> = if !blocking {
        Some(Arc::new(MutexSpin::new()))
    } else {
        Some(Arc::new(MutexBlocking::new()))
    };
    let mut process_inner = process.inner_exclusive_access();
    let id = if let Some(id) = process_inner
        .mutex_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.mutex_list[id] = mutex;
        id
    } else {
        process_inner.mutex_list.push(mutex);
        process_inner.mutex_list.len() - 1
    };
    if process_inner.deadlock_detect {
        if let Some(mut rm) = process_inner.resource_manager.take() {
            rm.sync_mutex_list(&process_inner.mutex_list);
            process_inner.resource_manager = Some(rm);
        }
    }
    id as isize
}
/// mutex lock syscall
pub fn sys_mutex_lock(mutex_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_lock",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let tid = current_tid();
    let process = current_process();
    {
        let mut inner = process.inner_exclusive_access();
        if inner.deadlock_detect {
            if let Some(rm) = inner.resource_manager.as_mut() {
                if mutex_id >= rm.mutex_check.num_resources()
                    || rm.mutex_check.total.get(mutex_id).copied().unwrap_or(0) == 0
                {
                    return -1;
                }
                match rm.mutex_check.try_acquire(tid, mutex_id, 1) {
                    Ok(AcquireResult::Granted) | Ok(AcquireResult::WillWait) => {}
                    Err(()) => return SYSCALL_DEADLOCK,
                }
            }
        }
    }
    let mutex = {
        let inner = process.inner_exclusive_access();
        Arc::clone(inner.mutex_list[mutex_id].as_ref().unwrap())
    };
    mutex.lock();
    if process.inner_exclusive_access().deadlock_detect {
        let mut inner = process.inner_exclusive_access();
        if let Some(rm) = inner.resource_manager.as_mut() {
            if rm.mutex_check.allocation(tid, mutex_id) == 0 {
                rm.mutex_check
                    .complete_acquire_after_wait(tid, mutex_id, 1);
            }
        }
    }
    0
}
/// mutex unlock syscall
pub fn sys_mutex_unlock(mutex_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_mutex_unlock",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let tid = current_tid();
    let process = current_process();
    let mutex = {
        let inner = process.inner_exclusive_access();
        Arc::clone(inner.mutex_list[mutex_id].as_ref().unwrap())
    };
    let handoff = mutex.deadlock_unlock_handoff_tid();
    mutex.unlock();
    if process.inner_exclusive_access().deadlock_detect {
        let mut inner = process.inner_exclusive_access();
        if let Some(rm) = inner.resource_manager.as_mut() {
            if mutex_id < rm.mutex_check.num_resources() && rm.mutex_check.total[mutex_id] > 0 {
                if let Some(ntid) = handoff {
                    rm.mutex_check
                        .transfer_allocation(mutex_id, tid, ntid, 1);
                } else {
                    rm.mutex_check.release(tid, mutex_id, 1);
                }
            }
        }
    }
    0
}
/// semaphore create syscall
pub fn sys_semaphore_create(res_count: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let id = if let Some(id) = process_inner
        .semaphore_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.semaphore_list[id] = Some(Arc::new(Semaphore::new(res_count)));
        id
    } else {
        process_inner
            .semaphore_list
            .push(Some(Arc::new(Semaphore::new(res_count))));
        process_inner.semaphore_list.len() - 1
    };
    if process_inner.deadlock_detect {
        if let Some(mut rm) = process_inner.resource_manager.take() {
            rm.sync_semaphore_list(&process_inner.semaphore_list);
            process_inner.resource_manager = Some(rm);
        }
    }
    id as isize
}
/// semaphore up syscall
pub fn sys_semaphore_up(sem_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_up",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    //println!("sys_semaphore_up for sem {}", sem_id);
    let tid = current_tid();
    let process = current_process();
    let sem = {
        let inner = process.inner_exclusive_access();
        Arc::clone(inner.semaphore_list[sem_id].as_ref().unwrap())
    };
    let waiter = sem.peek_waiter_tid();
    sem.up();
    if process.inner_exclusive_access().deadlock_detect {
        let mut inner = process.inner_exclusive_access();
        if let Some(rm) = inner.resource_manager.as_mut() {
            if sem_id < rm.sem_check.num_resources() && rm.sem_check.total[sem_id] > 0 {
                let caller_holds = rm.sem_check.allocation(tid, sem_id) > 0;
                if let Some(wt) = waiter {
                    if caller_holds {
                        rm.sem_check.transfer_allocation(sem_id, tid, wt, 1);
                    } else {
                        rm.sem_check.semaphore_up_wake(sem_id, wt);
                    }
                } else if caller_holds {
                    rm.sem_check.release(tid, sem_id, 1);
                } else {
                    rm.sem_check.semaphore_up_no_waiter(sem_id, 1);
                }
            }
        }
    }
    //println!("sys_semaphore_up for sem {} done", sem_id);
    0
}
/// semaphore down syscall
pub fn sys_semaphore_down(sem_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_semaphore_down",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let tid = current_tid();
    let process = current_process();
    {
        //println!("sys_semaphore_down for sem {}", sem_id);
        let mut inner = process.inner_exclusive_access();
        if inner.deadlock_detect {
            if let Some(rm) = inner.resource_manager.as_mut() {
                if sem_id >= rm.sem_check.num_resources() {
                    return -1;
                }
                // total==0: e.g. ch8b_sync_sem barrier; skip banker, real sem still works.
                let tcol = rm.sem_check.total.get(sem_id).copied().unwrap_or(0);
                if tcol > 0 {
                    match rm.sem_check.try_acquire(tid, sem_id, 1) {
                        Ok(AcquireResult::Granted) | Ok(AcquireResult::WillWait) => {}
                        Err(()) => return SYSCALL_DEADLOCK,
                    }
                }
            }
        }
    }
    let sem = {
        let inner = process.inner_exclusive_access();
        Arc::clone(inner.semaphore_list[sem_id].as_ref().unwrap())
    };
    sem.down();
    if process.inner_exclusive_access().deadlock_detect {
        let mut inner = process.inner_exclusive_access();
        if let Some(rm) = inner.resource_manager.as_mut() {
            let tcol = rm.sem_check.total.get(sem_id).copied().unwrap_or(0);
            if tcol > 0 && rm.sem_check.allocation(tid, sem_id) == 0 {
                rm.sem_check.complete_acquire_after_wait(tid, sem_id, 1);
            }
        }
    }
    //println!("sys_semaphore_down for sem {} done", sem_id);
    0
}
/// condvar create syscall
pub fn sys_condvar_create() -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_create",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let id = if let Some(id) = process_inner
        .condvar_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.condvar_list[id] = Some(Arc::new(Condvar::new()));
        id
    } else {
        process_inner
            .condvar_list
            .push(Some(Arc::new(Condvar::new())));
        process_inner.condvar_list.len() - 1
    };
    id as isize
}
/// condvar signal syscall
pub fn sys_condvar_signal(condvar_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_signal",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let condvar = Arc::clone(process_inner.condvar_list[condvar_id].as_ref().unwrap());
    drop(process_inner);
    condvar.signal();
    0
}
/// condvar wait syscall
pub fn sys_condvar_wait(condvar_id: usize, mutex_id: usize) -> isize {
    trace!(
        "kernel:pid[{}] tid[{}] sys_condvar_wait",
        current_task().unwrap().process.upgrade().unwrap().getpid(),
        current_task()
            .unwrap()
            .inner_exclusive_access()
            .res
            .as_ref()
            .unwrap()
            .tid
    );
    let tid = current_tid();
    let process = current_process();
    let dl = process.inner_exclusive_access().deadlock_detect;
    let process_inner = process.inner_exclusive_access();
    let condvar = Arc::clone(process_inner.condvar_list[condvar_id].as_ref().unwrap());
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    drop(process_inner);

    if !dl {
        condvar.wait(mutex);
        return 0;
    }

    // Mirror Condvar::wait but keep mutex shadow aligned with unlock / re-lock.
    let handoff = mutex.deadlock_unlock_handoff_tid();
    mutex.unlock();
    {
        let mut inner = process.inner_exclusive_access();
        if let Some(rm) = inner.resource_manager.as_mut() {
            if mutex_id < rm.mutex_check.num_resources() && rm.mutex_check.total[mutex_id] > 0 {
                if let Some(ntid) = handoff {
                    rm.mutex_check
                        .transfer_allocation(mutex_id, tid, ntid, 1);
                } else {
                    rm.mutex_check.release(tid, mutex_id, 1);
                }
            }
        }
    }
    {
        let mut inner = condvar.inner.exclusive_access();
        inner.wait_queue.push_back(current_task().unwrap());
    }
    block_current_and_run_next();

    {
        let mut inner = process.inner_exclusive_access();
        if let Some(rm) = inner.resource_manager.as_mut() {
            if mutex_id < rm.mutex_check.num_resources() && rm.mutex_check.total[mutex_id] > 0 {
                match rm.mutex_check.try_acquire(tid, mutex_id, 1) {
                    Ok(AcquireResult::Granted) | Ok(AcquireResult::WillWait) => {}
                    Err(()) => return SYSCALL_DEADLOCK,
                }
            }
        }
    }
    mutex.lock();
    {
        let mut inner = process.inner_exclusive_access();
        if let Some(rm) = inner.resource_manager.as_mut() {
            if rm.mutex_check.allocation(tid, mutex_id) == 0 {
                rm.mutex_check
                    .complete_acquire_after_wait(tid, mutex_id, 1);
            }
        }
    }
    0
}
/// enable deadlock detection syscall
pub fn sys_enable_deadlock_detect(enabled: usize) -> isize {
    trace!("kernel: sys_enable_deadlock_detect");
    if enabled != 0 && enabled != 1 {
        return -1;
    }
    let process = current_process();
    let mut inner = process.inner_exclusive_access();
    if enabled == 0 {
        inner.deadlock_detect = false;
        inner.resource_manager = None;
        return 0;
    }
    inner.deadlock_detect = true;
    inner.resource_manager = Some(ResourceManager::from_process_inner(&*inner));
    0
}
