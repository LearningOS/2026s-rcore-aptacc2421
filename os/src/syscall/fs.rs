//! File and filesystem-related syscalls
use crate::fs::{open_file, OpenFlags, Stat, unlink_file, link_file};
use crate::mm::{translated_byte_buffer, translated_str, UserBuffer};
use crate::task::{current_task, current_user_token};

pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> isize {
    trace!("kernel:pid[{}] sys_write", current_task().unwrap().pid.0);
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        if !file.writable() {
            return -1;
        }
        let file = file.clone();
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        file.write(UserBuffer::new(translated_byte_buffer(token, buf, len))) as isize
    } else {
        -1
    }
}

pub fn sys_read(fd: usize, buf: *const u8, len: usize) -> isize {
    trace!("kernel:pid[{}] sys_read", current_task().unwrap().pid.0);
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        let file = file.clone();
        if !file.readable() {
            return -1;
        }
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        trace!("kernel: sys_read .. file.read");
        file.read(UserBuffer::new(translated_byte_buffer(token, buf, len))) as isize
    } else {
        -1
    }
}

pub fn sys_open(path: *const u8, flags: u32) -> isize {
    trace!("kernel:pid[{}] sys_open", current_task().unwrap().pid.0);
    let task = current_task().unwrap();
    let token = current_user_token();
    let path = translated_str(token, path);
    if let Some(inode) = open_file(path.as_str(), OpenFlags::from_bits(flags).unwrap()) {
        let mut inner = task.inner_exclusive_access();
        let fd = inner.alloc_fd();
        inner.fd_table[fd] = Some(inode);
        fd as isize
    } else {
        -1
    }
}

pub fn sys_close(fd: usize) -> isize {
    trace!("kernel:pid[{}] sys_close", current_task().unwrap().pid.0);
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if inner.fd_table[fd].is_none() {
        return -1;
    }
    inner.fd_table[fd].take();
    0
}

/// YOUR JOB: Implement fstat.
/*
功能：获取文件状态。

Ｃ接口： int fstat(int fd, struct Stat* st)

Rust 接口： fn fstat(fd: i32, st: *mut Stat) -> i32

参数：
fd: 文件描述符

st: 文件状态结构体
*/
pub fn sys_fstat(fd: usize, st: *mut Stat) -> isize {
    trace!(
        "kernel:pid[{}] sys_fstat",
        current_task().unwrap().pid.0
    );
    //-1
    let token = current_user_token();
    let task = current_task().unwrap();
    let inner = task.inner_exclusive_access();
    if fd >= inner.fd_table.len() {
        return -1;
    }
    if let Some(file) = &inner.fd_table[fd] {
        let file = file.clone();
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        // user buffer for stat need to translate in kernel space, so we can write to it directly
        let stat = file.stat();
        // translate st to kernel space for use
        let stat_bytes = unsafe {
            core::slice::from_raw_parts(
                (&stat as *const Stat) as *const u8,
                core::mem::size_of::<Stat>(),
            )
        };
        let mut bufs = translated_byte_buffer(
            token,
            st as *const u8,
            core::mem::size_of::<Stat>(),
        );
        let mut pos = 0usize;
        for chunk in bufs.iter_mut() {
            if pos >= stat_bytes.len() {
                break;
            }
            let n = chunk.len().min(stat_bytes.len() - pos);
            chunk[..n].copy_from_slice(&stat_bytes[pos..pos + n]);
            pos += n;
        }
        assert_eq!(pos, stat_bytes.len());
        0
    } else {
        -1
    }

}

/// YOUR JOB: Implement linkat.
/*
参数：
olddirfd，newdirfd: 仅为了兼容性考虑，本次实验中始终为 AT_FDCWD (-100)，可以忽略。

flags: 仅为了兼容性考虑，本次实验中始终为 0，可以忽略。

oldpath：原有文件路径

newpath: 新的链接文件路径。

说明：
为了方便，不考虑新文件路径已经存在的情况（属于未定义行为）。除非出现新旧名字一致的情况，此时需要返回-1。

返回值：如果出现了错误则返回 -1，否则返回 0。

可能的错误
链接同名文件。
*/
pub fn sys_linkat(_old_name: *const u8, _new_name: *const u8) -> isize {
    trace!(
        "kernel:pid[{}] sys_linkat",
        current_task().unwrap().pid.0
    );
    //-1
    let token = current_user_token();
    let old_name = translated_str(token, _old_name);
    let new_name = translated_str(token, _new_name);
    if old_name == new_name {
        return -1;
    }
    if link_file(old_name.as_str(), new_name.as_str()) {
        0
    } else {
        -1
    }
}

/// YOUR JOB: Implement unlinkat.
/*
功能：取消一个文件路径到文件的链接, unlinkat标准接口 。

Ｃ接口： int unlinkat(int dirfd, char* path, unsigned int flags)

Rust 接口： fn unlinkat(dirfd: i32, path: *const u8, flags: u32) -> i32

参数：
dirfd: 仅为了兼容性考虑，本次实验中始终为 AT_FDCWD (-100)，可以忽略。

flags: 仅为了兼容性考虑，本次实验中始终为 0，可以忽略。

path：文件路径。

说明：
注意考虑使用 unlink 彻底删除文件的情况，此时需要回收inode以及它对应的数据块。

返回值：如果出现了错误则返回 -1，否则返回 0。

可能的错误
文件不存在。
*/
pub fn sys_unlinkat(_name: *const u8) -> isize {
    trace!(
        "kernel:pid[{}] sys_unlinkat",
        current_task().unwrap().pid.0
    );
    //-1
    let token = current_user_token();
    let name = translated_str(token, _name);
    if unlink_file(name.as_str()) {
        0
    } else {
        -1
    }
}
