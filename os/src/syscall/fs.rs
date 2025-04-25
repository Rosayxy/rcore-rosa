//! File and filesystem-related syscalls

use crate::fs::File;
use crate::fs::{open_file, OpenFlags, Stat};
use crate::fs::inode::ROOT_INODE;
use crate::mm::{translated_byte_buffer, translated_str, UserBuffer};
use crate::syscall::process::virt_to_phys;
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
    let mut path = translated_str(token, path);
    // try find real path
    let binding = task.inner_exclusive_access();
    let real_path = binding.links.iter().find(|x| x.0 == path);
    if real_path.is_some() {
        path = real_path.unwrap().1.clone();
    }
    drop(binding); // 试一下手动 drop
    if let Some(inode) = open_file(path.as_str(), OpenFlags::from_bits(flags).unwrap()) {
        let mut inner = task.inner_exclusive_access();
        let fd = inner.alloc_fd();
        if flags & OpenFlags::CREATE.bits() != 0{
            inner.links.push((path.clone(), path.clone()));
        }
        inner.fd_table[fd] = Some(inode.clone());
        if inner.fd_helper.len() <= fd {
            inner.fd_helper.resize(fd + 1, None);
        }
        inner.fd_helper[fd] = Some(path.clone());
        // initialize nlink
        let cnt = inner.links.iter().filter(|x| x.1 == path).count();
        if cnt - 1> 0 {
            inode.incl_nlink(cnt - 1);
        }
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
    // 维护 fd_helper
    inner.fd_helper[fd].take();
    0
}

/// YOUR JOB: Implement fstat.
pub fn sys_fstat(_fd: usize, _st: *mut Stat) -> isize {
    trace!(
        "kernel:pid[{}] sys_fstat",
        current_task().unwrap().pid.0
    );
    let task = current_task().unwrap();
    
    // First, check if fd is valid and get the file
    let file_opt = {
        let inner = task.inner_exclusive_access();
        if _fd >= inner.fd_table.len() {
            return -1;
        }
        
        // Clone the file if it exists
        let opt = inner.fd_table.get(_fd).cloned().unwrap();
        drop(inner);
        opt
    };
    
    // Now work with the file outside the exclusive access
    if let Some(file) = file_opt {
        // Get the stat without holding the lock
        let stat = file.fstat();
        
        // Convert and update the stat struct
        let st_phys = virt_to_phys(_st as usize);
        let st = unsafe { &mut *(st_phys as *mut Stat) };
        st.dev = stat.dev;
        st.ino = stat.ino;
        st.mode = stat.mode;
        st.nlink = stat.nlink;
        
        // Store the pointer in a separate critical section
        let st_ptr = st as *const Stat as usize;
        
        // Now get exclusive access again to update fstat_ptrs
        let mut inner = task.inner_exclusive_access();
        if inner.fstat_ptrs.len() <= _fd {
            inner.fstat_ptrs.resize(_fd + 1, None);
        }
        inner.fstat_ptrs[_fd] = Some(st_ptr);
        
        return 0;
    }
    
    -1
}

/// YOUR JOB: Implement linkat.
pub fn sys_linkat(_old_name: *const u8, _new_name: *const u8) -> isize {
    trace!(
        "kernel:pid[{}] sys_linkat NOT IMPLEMENTED",
        current_task().unwrap().pid.0
    );
    let old_name_str = translated_str(current_user_token(), _old_name);
    let new_name_str = translated_str(current_user_token(), _new_name);
    if old_name_str == new_name_str {
        return -1;
    }
    
    let task = current_task().unwrap();
    {
        let mut inner = task.inner_exclusive_access();
        inner.links.push((new_name_str.clone(), old_name_str.clone()));
    }
    
    // Step 2: Find the fd
    let (fd_opt, file_opt) = {
        let inner = task.inner_exclusive_access();
        
        // Find the fd corresponding to old_name_str
        let fd_opt = inner.fd_helper.iter().enumerate()
            .find(|(_idx, x)| {
                if let Some(name) = x {
                    return *name == old_name_str;
                }
                false
            })
            .map(|(idx, _)| idx);
        
        if fd_opt.is_none() {
            return 0;
        }
        
        let fd = fd_opt.unwrap();
        if fd >= inner.fd_table.len() {
            return -1;
        }
        
        // Get the file and its fstat pointer (if any)
        let file_opt = inner.fd_table.get(fd).cloned().unwrap();
        let st_ptr_opt = if inner.fstat_ptrs.len() > fd {
            inner.fstat_ptrs[fd]
        } else {
            None
        };
        
        (Some((fd, st_ptr_opt)), file_opt)
    };
    
    // Step 3: Update file and stat outside of the exclusive access
    if let Some(file) = file_opt {
        // Increment the file's link count
        file.incl_nlink(1);
        
        // Update stat if it exists
        if let Some((_fd, Some(st_ptr))) = fd_opt {
            let st_phys = st_ptr;
            let st = unsafe { &mut *(st_phys as *mut Stat) };
            st.nlink += 1;
        }
    }
    
    0
}

/// YOUR JOB: Implement unlinkat.
/// 我们 task 结构体里面主要是 links 和 fd_helper 需要维护
/// 我们先判断这个 unlink 的项是否是在 fd_helper 里面，如果不在的话，就只用在 links 中删除对应项就行
/// 否则，用 links 里面的第一项取代 fd_helper 里面的 string 然后更新 links
pub fn sys_unlinkat(_name: *const u8) -> isize {
    println!("kernel:pid[{}] sys_unlinkat", current_task().unwrap().pid.0);
    let name_str = translated_str(current_user_token(), _name);
    let task = current_task().unwrap();
    let mut inner = task.inner_exclusive_access();
    let links_vec = inner.links.clone();
    let mut fd = None;
    // find if appears in fd_helper
    println!("rosa: fd_helper = {:?}", inner.fd_helper);
    // first find true path in link
    let mut true_path = None;
    for x in links_vec.iter() {
        if x.0 == name_str {
            true_path = Some(x.1.clone());
            break;
        }
    }
    if true_path.is_none() {
        return -1;
    }
    for (idx, x) in inner.fd_helper.iter().enumerate() {
        if x.is_none() {
            continue;
        }
        if *x.as_ref().unwrap() == *true_path.as_ref().unwrap() {
            fd = Some(idx);
            println!("rosa: finding in fd_helper");
            break;
        }
    }
    // 直接在 links 中删除
    let mut is_del = false;
    for (idx, x) in links_vec.iter().enumerate() {
        if x.0 == name_str {
            inner.links.remove(idx);
            // update the nlink
            if let Some(fd_idx) = fd {
                let file = inner.fd_table[fd_idx].clone();
                if let Some(file) = file {
                    file.decl_nlink(1);
                    // check if has fstat
                    if inner.fstat_ptrs.len() > idx {
                        if let Some(st_ptr) = inner.fstat_ptrs[idx] {
                            let st_phys = st_ptr;
                            let st = unsafe { &mut *(st_phys as *mut Stat) };
                            st.nlink -= 1;
                        }
                    }
                    println!("rosa: decl_nlink");
                }
            }
            is_del = true;
        }
    }
    // check links
    let link_cnt = inner.links.iter().filter(|x| x.1 == *true_path.as_ref().unwrap()).count();
    if link_cnt == 0 {
        // 析构
        ROOT_INODE.remove_inode(name_str.as_str());
    }
    return if is_del { 0 } else { -1 };
}
