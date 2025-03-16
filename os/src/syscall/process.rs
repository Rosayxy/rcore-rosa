//! Process management syscalls
use crate::{
    config::PAGE_SIZE, mm::{MapPermission, VirtAddr, VirtPageNum}, task::{change_program_brk, current_user_token, exit_current_and_run_next, get_current_ranges, insert_framed_area, suspend_current_and_run_next, unmap_framed_area}, timer::get_time_us, trace_array::read_from_array
};

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
    println!("mmap: {} {} {:?}",_start, end, map_permission);
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
    trace!("kernel: sys_sbrk");
    if let Some(old_brk) = change_program_brk(size) {
        old_brk as isize
    } else {
        -1
    }
}
