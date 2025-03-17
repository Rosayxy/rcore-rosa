//! Process management syscalls
//!
use alloc::sync::Arc;

use crate::{
    config::MAX_SYSCALL_NUM,
    fs::{open_file, OpenFlags},
    mm::{translated_refmut, translated_str},
    task::{
        add_task, current_task, current_user_token, exit_current_and_run_next,
        suspend_current_and_run_next, TaskStatus,
    },
};
use crate::{
    config::PAGE_SIZE, mm::{MapPermission, VirtAddr, VirtPageNum}, task::{change_program_brk, get_current_ranges, insert_framed_area, unmap_framed_area}, timer::get_time_us, trace_array::read_from_array
};

use super::get_trace_idx;

#[repr(C)]
#[derive(Debug)]
pub struct TimeVal {
    pub sec: usize,
    pub usec: usize,
}

/// Task information
#[allow(dead_code)]
pub struct TaskInfo {
    /// Task status in it's life cycle
    status: TaskStatus,
    /// The numbers of syscall called by task
    syscall_times: [u32; MAX_SYSCALL_NUM],
    /// Total running time of task
    time: usize,
}

pub fn sys_exit(exit_code: i32) -> ! {
    trace!("kernel:pid[{}] sys_exit", current_task().unwrap().pid.0);
    exit_current_and_run_next(exit_code);
    panic!("Unreachable in sys_exit!");
}

pub fn sys_yield() -> isize {
    //trace!("kernel: sys_yield");
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
    if let Some(app_inode) = open_file(path.as_str(), OpenFlags::RDONLY) {
        let all_data = app_inode.read_all();
        let task = current_task().unwrap();
        task.exec(all_data.as_slice());
        0
    } else {
        -1
    }
}

/// If there is not a child process whose pid is same as given, return -1.
/// Else if there is a child process but it is still running, return -2.
pub fn sys_waitpid(pid: isize, exit_code_ptr: *mut i32) -> isize {
    //trace!("kernel: sys_waitpid");
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
pub fn virt_to_phys(virt:usize)->*mut u8{
    let vpn = virt/PAGE_SIZE;
    let token = current_user_token();
    let page_table = crate::mm::page_table::PageTable::from_token(token);
    let ppn = page_table.translate(VirtPageNum::from(vpn)).unwrap().ppn();
    let offset = virt % PAGE_SIZE;
    let ppn_ = ppn.get_bytes_array();
    let ppn = ppn_.as_mut_ptr();
    let phys = unsafe { ppn.add(offset) as *mut u8 };
    phys
}

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?

pub fn sys_get_time(ts: *mut TimeVal, _tz: usize) -> isize {
    trace!("kernel: sys_get_time");
    let us = get_time_us();
    // convert the address to physical address
    let vpn = ts as usize / PAGE_SIZE;
    let token = current_user_token();
    let page_table = crate::mm::page_table::PageTable::from_token(token);
    let ppn = page_table.translate(VirtPageNum::from(vpn)).unwrap().ppn();
    let offset = ts as usize % PAGE_SIZE;
    let ppn_ = ppn.get_bytes_array();
    let ppn = ppn_.as_mut_ptr();
    let ts_phys = unsafe { ppn.add(offset) as *mut TimeVal };
    unsafe {
        *ts_phys = TimeVal {
            sec: us / 1_000_000,
            usec: us % 1_000_000,
        };
    }
    0
}
/// check the given address in trace is accessible
pub fn is_legal_access(address:usize,read_perm:bool,write_perm:bool)->bool{
    let vpn = address/PAGE_SIZE;
    let ranges = crate::task::get_ranges();
    for range in ranges.iter(){
        if range.0 <= vpn && vpn < range.1{
            if read_perm && range.2.contains(MapPermission::R) || write_perm && range.2.contains(MapPermission::W){
                return true;
            }
        }
    }
    false
}
/// TODO: Finish sys_trace to pass testcases
/// HINT: You might reimplement it with virtual memory management.
pub fn sys_trace(_trace_request: usize, _id: usize, _data: usize) -> isize {
    // check address first
    if _trace_request == 0 { // trace read
        if !is_legal_access(_id, true, false){
            return -1;
        }
    }else if _trace_request == 1{ // trace write
        if !is_legal_access(_id, false, true){
            return -1;
        }
    }
    match _trace_request{
        0=>{
            // id 应被视作 *const u8 ，表示读取当前任务 id 地址处一个字节的无符号整数值 convert id to physical address
            let id = virt_to_phys(_id);
            return unsafe { *id as isize };
        }
        1=>{
            // id 应被视作 *const u8 ，表示写入 data （作为 u8，即只考虑最低位的一个字节）到该用户程序 id 地址处。返回值应为0
            let id = virt_to_phys(_id);
            // write data to id
            unsafe { *id = _data as u8 };
            return 0;
        }
        2=>{
            return read_from_array(get_trace_idx(_id)).unwrap();
        }
        _=>{
            -1
        }
    }
}

/// YOUR JOB: Finish sys_task_info to pass testcases
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TaskInfo`] is splitted by two pages ?
pub fn sys_task_info(_ti: *mut TaskInfo) -> isize {
    trace!(
        "kernel:pid[{}] sys_task_info NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    -1
}

// YOUR JOB: Implement mmap.
pub fn sys_mmap(_start: usize, _len: usize, _prot: usize) -> isize {
    // check legal
    if _len ==0{
        return 0;
    }
    if _start%PAGE_SIZE != 0{
        return -1;
    }
    if (_prot & !0x7 != 0) || (_prot & 0x7 == 0){
        return -1;
    }
    let ceiled_len = (_len + PAGE_SIZE - 1) / PAGE_SIZE * PAGE_SIZE;
    // check if mapped
    let ranges = get_current_ranges();
    for range in ranges.iter(){
        if range.0 <= _start/PAGE_SIZE && _start/PAGE_SIZE < range.1{
            return -1;
        }
        // check end of range
        if range.0 < (_start + ceiled_len)/PAGE_SIZE && (_start + ceiled_len)/PAGE_SIZE <= range.1{
            return -1;
        }
    }
    
    // create mapping
    //  TODO if the address is mapped, check its length and prot, change the prot if needed, using the get_ranges
    let mut map_permission = MapPermission::U;
    if _prot & 0x1 != 0{
        map_permission |= MapPermission::R;
    }
    if _prot & 0x2 != 0{
        map_permission |= MapPermission::W;
    }
    if _prot & 0x4 != 0{
        map_permission |= MapPermission::X;
    }
    let end = _start + ceiled_len;
    // insert in memory set 和 get_ranges 一样一层层加吧
    // println!("mmap: {} {} {:?}",_start, end, map_permission);
    // get current task
    insert_framed_area(VirtAddr(_start), VirtAddr(end), map_permission);
    0
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    if _start % PAGE_SIZE != 0 {
        return -1;
    }
    let ceiled_len = (_len + PAGE_SIZE - 1) / PAGE_SIZE * PAGE_SIZE;
    let end = _start + ceiled_len;
    // check unmap ranges must be legal
    let ranges = get_current_ranges();
    let mut is_legal = false;
    for range in ranges.iter(){
        if range.0 == _start/PAGE_SIZE && range.1 == end/PAGE_SIZE{
            is_legal = true;
            break;
        }
    }
    if !is_legal{
        return -1;
    }
    unmap_framed_area(VirtAddr(_start), VirtAddr(end));
    return 0;
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
pub fn sys_spawn(_path: *const u8) -> isize {
    trace!(
        "kernel:pid[{}] sys_spawn NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    -1
}

// YOUR JOB: Set task priority.
pub fn sys_set_priority(_prio: isize) -> isize {
    trace!(
        "kernel:pid[{}] sys_set_priority NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    -1
}
