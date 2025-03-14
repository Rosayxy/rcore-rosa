//! Implementation of syscalls
//!
//! The single entry point to all system calls, [`syscall()`], is called
//! whenever userspace wishes to perform a system call using the `ecall`
//! instruction. In this case, the processor raises an 'Environment call from
//! U-mode' exception, which is handled as one of the cases in
//! [`crate::trap::trap_handler`].
//!
//! For clarity, each single syscall is implemented as its own function, named
//! `sys_` then the name of the syscall. You can find functions like this in
//! submodules, and you should also implement syscalls this way.
const SYSCALL_WRITE: usize = 64;
/// exit syscall
const SYSCALL_EXIT: usize = 93;
/// yield syscall
const SYSCALL_YIELD: usize = 124;
/// gettime syscall
const SYSCALL_GET_TIME: usize = 169;
/// sbrk syscall
const SYSCALL_SBRK: usize = 214;
/// munmap syscall
const SYSCALL_MUNMAP: usize = 215;
/// mmap syscall
const SYSCALL_MMAP: usize = 222;
/// trace syscall
const SYSCALL_TRACE: usize = 410;

mod fs;
mod process;

use fs::*;
use process::*;

use crate::trace_array::incl_array;

/// get the index of the trace
pub fn get_trace_idx(ty: usize) -> usize {
    match ty {
        SYSCALL_WRITE => 0,
        SYSCALL_EXIT => 1,
        SYSCALL_YIELD => 2,
        SYSCALL_GET_TIME => 3,
        SYSCALL_TRACE => 4,
        SYSCALL_SBRK => 5,
        SYSCALL_MMAP => 6,
        SYSCALL_MUNMAP => 7,
        _ => panic!("Unsupported trace type: {}", ty),
    }
}

/// handle syscall exception with `syscall_id` and other arguments
pub fn syscall(syscall_id: usize, args: [usize; 3]) -> isize {
    incl_array(get_trace_idx(syscall_id)).unwrap();
    match syscall_id {
        SYSCALL_WRITE => {
            sys_write(args[0], args[1] as *const u8, args[2])},
        SYSCALL_EXIT => {
            sys_exit(args[0] as i32)},
        SYSCALL_YIELD => {
            sys_yield()},
        SYSCALL_GET_TIME => sys_get_time(args[0] as *mut TimeVal, args[1]),
        SYSCALL_TRACE => sys_trace(args[0], args[1], args[2]),
        SYSCALL_MMAP => sys_mmap(args[0], args[1], args[2]),
        SYSCALL_MUNMAP => sys_munmap(args[0], args[1]),
        SYSCALL_SBRK => sys_sbrk(args[0] as i32),
        _ => panic!("Unsupported syscall_id: {}", syscall_id),
    }
}
