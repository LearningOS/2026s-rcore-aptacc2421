//! Process management syscalls
use crate::mm::translated_byte_buffer;
use crate::task::{
    change_program_brk,
    current_task,
    current_user_token,
    exit_current_and_run_next,
    suspend_current_and_run_next,
};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// task exits and submit an exit code
pub fn sys_exit(_exit_code: i32) -> ! {
    trace!("kernel: sys_exit");
    exit_current_and_run_next();
    panic!("Unreachable in sys_exit!");
}

/// current task gives up resources for other tasks
pub fn sys_yield() -> isize {
    trace!("kernel: sys_yield");
    suspend_current_and_run_next();
    0
}

/// get time with second and microsecond
pub fn sys_get_time(_ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    println!("kernel: sys_get_time");
    if _ts.is_null() {
        return -1;
    }

    let buffers = translated_byte_buffer(
        current_user_token(),
        _ts as *const u8,
        core::mem::size_of::<TimeVal>(),
    );
    if buffers.is_empty() {
        return -1;
    }

    let msec = crate::timer::get_time_ms();
    let sec = msec / 1000;
    let usec = (msec % 1000) * 1000;
    let time_val = TimeVal { sec, usec };
    let time_val_bytes = unsafe {
        core::slice::from_raw_parts(
            (&time_val as *const TimeVal) as *const u8,
            core::mem::size_of::<TimeVal>(),
        )
    };

    let mut copied = 0;
    for page in buffers.into_iter() {
        let n = core::cmp::min(page.len(), time_val_bytes.len() - copied);
        page[..n].copy_from_slice(&time_val_bytes[copied..copied + n]);
        copied += n;
        if copied >= time_val_bytes.len() {
            break;
        }
    }

    0
}

pub fn sys_trace(_trace_request: usize, _id: usize, _data: usize) -> isize {
    trace!("kernel: sys_trace");
    println!("kernel: sys_trace: request={}, id={}, data={}", _trace_request, _id, _data);
    match _trace_request {
        0 => {
            let buffers = translated_byte_buffer(current_user_token(), _id as *const u8, 1);
            if let Some(page) = buffers.first() {
                *page.get(0).unwrap_or(&0) as isize
            } else {
                -1
            }
        }
        1 => {
            let mut buffers = translated_byte_buffer(current_user_token(), _id as *mut u8, 1);
            if let Some(page) = buffers.first_mut() {
                if let Some(byte) = page.get_mut(0) {
                    *byte = _data as u8;
                    return 0;
                }
            }
            -1
        }
        2 => {
            if let Some(task) = current_task() {
                if _id < task.syscall_counts.len() {
                    task.syscall_counts[_id] as isize
                } else {
                    -1
                }
            } else {
                -1
            }
        }
        _ => -1,
    }
}

pub fn sys_mmap(_start: usize, _len: usize, _prot: usize) -> isize {
    trace!("kernal: sys_mmap: start={:#x}, len={:#x}, prot={:#x}", _start, _len, _prot);
    println!("kernel: sys_mmap: start={:#x}, len={:#x}, prot={:#x}", _start, _len, _prot);
    use crate::config::PAGE_SIZE;
    use crate::mm::{MapPermission, VirtAddr, StepByOne};

    if _start % PAGE_SIZE != 0 {
        return -1;
    }
    if _prot & !0x7 != 0 {
        return -1;
    }
    if _prot & 0x7 == 0 {
        return -1;
    }
    if _len == 0 {
        return 0;
    }

    let start_va = VirtAddr::from(_start);
    let end_va = VirtAddr::from(_start + _len);
    let start_vpn = start_va.floor();
    let end_vpn = end_va.ceil();

    let mut permission = MapPermission::U;
    if _prot & 0x1 != 0 {
        permission |= MapPermission::R;
    }
    if _prot & 0x2 != 0 {
        permission |= MapPermission::W;
    }
    if _prot & 0x4 != 0 {
        permission |= MapPermission::X;
    }

    if let Some(task) = current_task() {
        let mut vpn = start_vpn;
        while vpn < end_vpn {
            if let Some(pte) = task.memory_set.translate(vpn) {
                if pte.is_valid() {
                    return -1;
                }
            }
            vpn.step();
        }
        if task
            .memory_set
            .try_insert_framed_area(start_va, end_va, permission)
        {
            0
        } else {
            -1
        }
    } else {
        -1
    }
}

pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    trace!("kernel: sys_munmap: start={:#x}, len={:#x}", _start, _len);
    println!("kernel: sys_munmap: start={:#x}, len={:#x}", _start, _len);
    use crate::config::PAGE_SIZE;
    use crate::mm::{VirtAddr, StepByOne};

    if _start % PAGE_SIZE != 0 {
        return -1;
    }
    if _len == 0 {
        return 0;
    }

    let start_va = VirtAddr::from(_start);
    let end_va = VirtAddr::from(_start + _len);
    let start_vpn = start_va.floor();
    let end_vpn = end_va.ceil();

    if let Some(task) = current_task() {
        let mut vpn = start_vpn;
        while vpn < end_vpn {
            match task.memory_set.translate(vpn) {
                Some(pte) if pte.is_valid() => {}
                _ => return -1,
            }
            vpn.step();
        }
        task.memory_set.unmap_range(start_vpn, end_vpn);
        0
    } else {
        -1
    }
}

pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel: sys_sbrk");
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}
