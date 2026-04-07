//! Process management syscalls
use alloc::sync::Arc;

use crate::{
    loader::get_app_data_by_name,
    mm::{translated_refmut, translated_str},
    task::{
        add_task, current_task, current_user_token, exit_current_and_run_next,
        suspend_current_and_run_next,
    },
};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(exit_code: i32) -> ! {
    trace!("kernel:pid[{}] sys_exit", current_task().unwrap().pid.0);
    exit_current_and_run_next(exit_code);
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel:pid[{}] sys_yield", current_task().unwrap().pid.0);
    suspend_current_and_run_next();
    0
}

pub fn sys_getpid() -> isize {
    trace!("kernel: sys_getpid pid:{}", current_task().unwrap().pid.0);
    current_task().unwrap().pid.0 as isize
}

pub fn sys_fork() -> isize {
    trace!("kernel:pid[{}] sys_fork", current_task().unwrap().pid.0);
    let current_task = current_task().unwrap();
    let new_task = current_task.fork();
    let new_pid = new_task.pid.0;
    // modify trap context of new_task, because it returns immediately after switching
    let trap_cx = new_task.inner_exclusive_access().get_trap_cx();
    // we do not have to move to next instruction since we have done it before
    // for child process, fork returns 0
    trap_cx.x[10] = 0;
    // add new task to scheduler
    add_task(new_task);
    new_pid as isize
}

pub fn sys_exec(path: *const u8) -> isize {
    trace!("kernel:pid[{}] sys_exec", current_task().unwrap().pid.0);
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(data) = get_app_data_by_name(path.as_str()) {
        let task = current_task().unwrap();
        task.exec(data);
        0
    } else {
        -1
    }
}

/// If there is not a child process whose pid is same as given, return -1.
/// Else if there is a child process but it is still running, return -2.
pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    trace!("kernel::pid[{}] sys_waitpid [{}]", current_task().unwrap().pid.0, pid);
    let task = current_task().unwrap();
    // find a child process

    // ---- access current PCB exclusively
    let mut inner = task.inner_exclusive_access();
    if !inner
        .children
        .iter()
        .any(|p| pid == -1 || pid as usize == p.getpid())
    {
        return -1;
        // ---- release current PCB
    }
    let pair = inner.children.iter().enumerate().find(|(_, p)| {
        // ++++ temporarily access child PCB exclusively
        p.inner_exclusive_access().is_zombie() && (pid == -1 || pid as usize == p.getpid())
        // ++++ release child PCB
    });
    if let Some((idx, _)) = pair {
        let child = inner.children.remove(idx);
        // confirm that child will be deallocated after being removed from children list
        assert_eq!(Arc::strong_count(&child), 1);
        let found_pid = child.getpid();
        // ++++ temporarily access child PCB exclusively
        let exit_code = child.inner_exclusive_access().exit_code;
        // ++++ release child PCB
        *translated_refmut(inner.memory_set.token(), exit_code_ptr) = exit_code;
        found_pid as isize
    } else {
        -2
    }
    // ---- release current PCB automatically
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
/* 
 * Note: This syscall shoud let ts = current time, and ignore tz for now.
 *       ts contains two fields: sec and usec, which are second and microsecond respectively.
 *       If the syscall is successfully executed, return 0. Otherwise, return -1.
 *
 * TODO: You should get current task first, and then write info in the address of ts.
 *       Just call the interface of current task to write info in the address of ts.
 *       You can get time by calling 'get_time_ms()' in 'timer.rs', and then convert it to second and microsecond.
 */
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!(
        "kernel:pid[{}] sys_get_time",
        current_task().unwrap().pid.0
    );
    //-1
    let usec = crate::timer::get_time_ms();
    let sec = usec / 1000000;
    *translated_refmut(current_user_token(), ts) = TimeVal { sec, usec };
    0
}

/// YOUR JOB: Implement mmap.
/*
 * sys_mmap - Allocate anonymous memory and map it to a virtual address.
 *
 * Arguments:
 *   start - Virtual start address. Must be page-aligned.
 *   len   - Size in bytes. May be zero.
 *   prot  - Memory protection bits:
 *           - Bit 0: readable
 *           - Bit 1: writable
 *           - Bit 2: executable
 *           All other bits must be zero.
 *
 * Behavior:
 *   Allocates physical memory of `len` bytes (rounded up to page size) and
 *   maps it to the virtual range [start, start + round_up(len)).
 *   The mapping is anonymous (not backed by a file).
 *   Page permissions are set according to prot.
 *
 * Returns:
 *   0 on success, -1 on error.
 *
 * Errors:
 *   - start is not page-aligned.
 *   - prot has any bits set outside bits 0-2.
 *   - (prot & 0x7) == 0  (no permission requested).
 *   - Any page in [start, start + round_up(len)) is already mapped.
 *   - Insufficient physical memory.
 *
 * Note:
 *   The implementation may ignore page allocation failures and does not need
 *   to handle partial cleanup (simplified for experiment).
 * 
 * Notice: if Any functions not belong to syscall should call interface of other module. 
 */
pub fn sys_mmap(start: usize, len: usize, prot: usize) -> isize {
    trace!(
        "kernel:pid[{}] sys_mmap",
        crate::task::current_task().unwrap().pid.0
    );
    //-1
    if !crate::mm::is_aligned_to_page_size(start) {
        return -1;
    }
    if prot & !0x7 != 0 || prot & 0x7 == 0 {
        return -1;
    }
    if len == 0 {
        return 0;
    }
    use crate::mm::VirtAddr;
    use crate::mm::MapPermission;
    let start_va = VirtAddr::from(start);
    let end_va = VirtAddr::from(start + len);

    let mut permission = MapPermission::empty();
    if prot & 0x1 != 0 {
        permission |= MapPermission::R;
    }
    if prot & 0x2 != 0 {
        permission |= MapPermission::W;
    }
    if prot & 0x4 != 0 {
        permission |= MapPermission::X;
    }
    permission |= MapPermission::U;

    (crate::mm::alloc_user_pages(start_va, end_va, permission)) as isize
}

/// YOUR JOB: Implement munmap.
/*
 * sys_munmap - Unmap anonymous memory previously mapped by mmap.
 * Syscall ID: 223 (example)
 *
 * Arguments:
 *   start - Virtual start address. Must be page-aligned.
 *   len   - Size in bytes. May be zero.
 *
 * Behavior:
 *   Unmaps the virtual range [start, start + round_up(len)) and frees the
 *   underlying physical pages. The range must exactly match a region created
 *   by a previous sys_mmap call.
 *
 * Returns:
 *   0 on success, -1 on error.
 *
 * Errors:
 *   - start is not page-aligned.
 *   - The specified range is not a currently mapped anonymous region
 *     (exact match required in this simplified version).
 *
 * Note:
 *   For simplicity, unmapping a partial region or merging/splitting regions
 *   is not required.
 */
pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    trace!(
        "kernel:pid[{}] sys_munmap",
        current_task().unwrap().pid.0
    );
    //-1
    use crate::mm::VirtAddr;

    if !crate::mm::is_aligned_to_page_size(_start) {
        return -1;
    }
    if _len == 0 {
        return 0;
    }
    let start_va = VirtAddr::from(_start);
    let end_va = VirtAddr::from(_start + _len);
    (crate::mm::dealloc_user_pages(start_va, end_va)) as isize

}

/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel:pid[{}] sys_sbrk", current_task().unwrap().pid.0);
    if let Some(old_brk) = current_task().unwrap().change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}

/// YOUR JOB: Implement spawn.
/// HINT: fork + exec =/= spawn
/*
 * sys_spawn - Create a new child process to execute the target program.
 *
 * Arguments:
 *   path - Pointer to a null-terminated path string of the program to execute.
 *
 * Behavior:
 *   Creates a new child process that loads and runs the executable specified
 *   by `path`. Returns the child process ID to the parent.
 *
 * Returns:
 *   Child process ID on success, -1 on error.
 *
 * Errors:
 *   - Invalid or inaccessible filename (e.g., path does not exist, cannot be read).
 *   - Insufficient resources to create a new process.
 *   - Other process creation failures.
 *
 * Note:
 *   The exact behavior (e.g., copying address space, using copy-on-write,
 *   or loading a new program image) follows the standard spawn semantics
 *   as defined in the experiment.
 */
pub fn sys_spawn(_path: *const u8) -> isize {
    if let Some(current_task) = current_task() {
        trace!("kernel:pid[{}] sys_spawn", current_task.pid.0);
        let token = current_user_token();
        let path = translated_str(token, _path);
        if let Some(data) = get_app_data_by_name(path.as_str()) {
            let task = Arc::new(crate::task::TaskControlBlock::new(data));
            let pid = task.getpid();

            {
                // 1. 父进程 -> 子进程
                let mut parent_inner = current_task.inner_exclusive_access();
                parent_inner.children.push(task.clone());
                drop(parent_inner);
                    
                // 2. 子进程 -> 父进程
                let mut child_inner = task.inner_exclusive_access();
                child_inner.parent = Some(Arc::downgrade(&current_task));
                drop(child_inner);
            }

            add_task(task);
            pid as isize  
        } else {
            -1
        }
    } else {
        -1
    }
}

// YOUR JOB: Set task priority.
// 设置当前进程优先级为 prio
// 参数：prio 进程优先级，要求 prio >= 2
// 返回值：如果输入合法则返回 prio，否则返回 -1
pub fn sys_set_priority(_prio: isize) -> isize {
    trace!(
        "kernel:pid[{}] sys_set_priority",
        current_task().unwrap().pid.0
    );
    //-1
    if _prio < 2 {
        return -1;
    }
    let current = current_task().unwrap();
    let mut inner = current.inner_exclusive_access();
    inner.priority_info.set_priority(_prio as usize);
    _prio
}
