//! Process management syscalls
use crate::{
    task::{exit_current_and_run_next, suspend_current_and_run_next},
    timer::get_time_us,
};
use super::{SYSCALL_WRITE, SYSCALL_EXIT, SYSCALL_YIELD, SYSCALL_GET_TIME, SYSCALL_TRACE};

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

#[derive(Clone, Copy, Debug)]
pub struct SyscallNum {
    pub write: usize,
    pub exit: usize,
    pub yield_: usize,
    pub get_time: usize,
    pub trace: usize,
}

impl SyscallNum {
    pub fn new() -> Self {
        Self {
            write: 0,
            exit: 0,
            yield_: 0,
            get_time: 0,
            trace: 0,
        }
    }

    pub fn get_syscall_num(&mut self, syscall_id: usize) -> usize {
        match syscall_id {
            SYSCALL_WRITE => self.write,
            SYSCALL_EXIT => self.exit,
            SYSCALL_YIELD => self.yield_,
            SYSCALL_GET_TIME => self.get_time,
            SYSCALL_TRACE => self.trace,
            _ => panic!("Unsupported syscall_id: {}", syscall_id),
        }
    }

    pub fn add_syscall_num(&mut self, syscall_id: usize) {
        match syscall_id {
            SYSCALL_WRITE => self.write += 1,
            SYSCALL_EXIT => self.exit += 1,
            SYSCALL_YIELD => self.yield_ += 1,
            SYSCALL_GET_TIME => self.get_time += 1,
            SYSCALL_TRACE => self.trace += 1,
            _ => panic!("Unsupported syscall_id: {}", syscall_id),
        }
    }
}



/// task exits and submit an exit code
pub fn sys_exit(exit_code: i32) -> ! {
    trace!("[kernel] Application exited with code {}", exit_code);
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
pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let us = get_time_us();
    unsafe {
        *ts = TimeVal {
            sec: us / 1_000_000,
            usec: us % 1_000_000,
        };
    }
    0
}

// TODO: implement the syscall
//if _trace_request == 0, then _id looked as *const u8, return the u8 in _id position
//if _trace_request == 1, then _id looked as *mut u8, write _data to the position and return 0
//if _trace_request == 2, then return the _id syscall times
//else return -1
pub fn sys_trace(_trace_request: usize, _id: usize, _data: usize) -> isize {
    trace!("kernel: sys_trace");
    //-1
    match _trace_request {
        0 => {
            let id = _id as *const u8;
            unsafe { *id as isize }
        }
        1 => {
            let id = _id as *mut u8;
            unsafe { *id = _data as u8 };
            0
        }
        2 => {
            let current = crate::task::TASK_MANAGER.current_task();
            crate::syscall::SYSCALL_NUM.exclusive_access()[current].get_syscall_num(_id) as isize
        }
        _ => -1,
    }
}
