//! Process management syscalls
use crate::{
    config::PAGE_SIZE, mm::{page_table::{map_virt_range, unmap_virt_range}, MapPermission, PTEFlags}, task::{change_program_brk, current_user_token, exit_current_and_run_next, suspend_current_and_run_next}, timer::get_time_us, trace_array::read_from_array
};
use crate::mm::frame_allocator::frame_alloc_size;
use super::get_trace_idx;

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

/// YOUR JOB: get time with second and microsecond
/// HINT: You might reimplement it with virtual memory management.
/// HINT: What if [`TimeVal`] is splitted by two pages ?
/// **这个还是用的 lab3 的 sys_get_time，如果有问题再修**
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
/// check the given address in trace is accessible
pub fn is_legal_access(address:usize,read_perm:bool,write_perm:bool)->bool{
    let ranges = crate::task::get_ranges();
    for range in ranges.iter(){
        if range.0 <= address && address < range.1{
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
    if _trace_request == 0 {
        if !is_legal_access(_id, true, false){
            return -1;
        }
    }else if _trace_request == 1{
        if !is_legal_access(_id, false, true){
            return -1;
        }
    }
    match _trace_request{
        0=>{
            // id 应被视作 *const u8 ，表示读取当前任务 id 地址处一个字节的无符号整数值
            let id = _id as *const u8;
            return unsafe { *id as isize };
        }
        1=>{
            // id 应被视作 *const u8 ，表示写入 data （作为 u8，即只考虑最低位的一个字节）到该用户程序 id 地址处。返回值应为0
            let id = _id as *mut u8;
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
    // create mapping
    if let Some(phys_start) = frame_alloc_size(ceiled_len){
        // 对每页进行映射
        let token = crate::task::current_user_token();
        // transform _start to vpn start
        let vpn_start = _start / PAGE_SIZE;
        map_virt_range(token, vpn_start, vpn_start + ceiled_len/PAGE_SIZE, phys_start,PTEFlags::from_bits(_prot as u8).unwrap());
        return 0;
    }else{
        return -1;
    }
}

// YOUR JOB: Implement munmap.
pub fn sys_munmap(_start: usize, _len: usize) -> isize {
    if _start % PAGE_SIZE != 0 {
        return -1;
    }
    let vpn_start = _start / PAGE_SIZE;
    let vpn_end = (_start + _len + PAGE_SIZE - 1) / PAGE_SIZE;
    unmap_virt_range(current_user_token(), vpn_start, vpn_end);
    return 0;
}
/// change data segment size
pub fn sys_sbrk(size: i32) -> isize {
    trace!("kernel: sys_sbrk");
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}
