use crate::sync::{Condvar, Mutex, MutexBlocking, MutexSpin, Semaphore};
// use crate::syscall::process;
use crate::task::{block_current_and_run_next, current_process, current_task};
use crate::timer::{add_timer, get_time_ms};
use alloc::sync::Arc;
use alloc::vec::Vec;
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
    let expire_ms = get_time_ms() + ms;
    let task = current_task().unwrap();
    add_timer(expire_ms, task);
    block_current_and_run_next();
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
    if let Some(id) = process_inner
        .mutex_list
        .iter()
        .enumerate()
        .find(|(_, item)| item.is_none())
        .map(|(id, _)| id)
    {
        process_inner.mutex_list[id] = mutex;
        let deadlock_detection_enabled = process_inner.deadlock_detection_enabled;
        if deadlock_detection_enabled {
            // If deadlock detection is enabled, we need to initialize the mutex
            // with a value of 1 to indicate that it is available.
            process_inner.avail_mutexes[id] = true;
        }
        id as isize
    } else {
        process_inner.mutex_list.push(mutex);
        let deadlock_detection_enabled = process_inner.deadlock_detection_enabled;
        if deadlock_detection_enabled {
            // If deadlock detection is enabled, we need to initialize the mutex
            // with a value of 1 to indicate that it is available.
            process_inner.avail_mutexes.push(true);
        }
        process_inner.mutex_list.len() as isize - 1
    }
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
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    let cur_tid = current_task()
        .unwrap()
        .inner_exclusive_access()
        .res
        .as_ref()
        .unwrap()
        .tid;
    // TODO 消灭后面这段的访存问题
    if process_inner.deadlock_detection_enabled {
        let mutex_cnt = process_inner.mutex_list.len();
        // check for deadlock
        // update need matrix
        let mut need_matrix = process_inner.need_matrix.clone();
        for line in need_matrix.iter_mut() {
            if line.len() < mutex_cnt {
                // extend the need matrix for current thread
                for _ in line.len()..mutex_cnt {
                    line.push(false);
                }
            }
        }

        need_matrix[cur_tid][mutex_id] = true;
        let mut allocation_matrix = process_inner.allocation_matrix.clone();
        for line in allocation_matrix.iter_mut() {
            if line.len() < mutex_cnt {
                // extend the allocation matrix for current thread
                for _ in line.len()..mutex_cnt {
                    line.push(false);
                }
            }
        }
        let mut deadlock_detected = false;
        let mut work_matrix = process_inner.avail_mutexes.clone();
        let tasks_num = process_inner.thread_count();
        let mut finish_matrix = Vec::new();
        let thread_cnt = process_inner.thread_count();
        for _ in 0..thread_cnt {
            finish_matrix.push(false);
        }
        // start detection
        while finish_matrix.iter().any(|&x| !x) {
            let mut found = false;
            for i in 0..tasks_num {
                if finish_matrix[i] {
                    continue;
                }
                // check if this thread can finish
                let mut can_finish = true;
                for j in 0..process_inner.mutex_list.len() {
                    if need_matrix[i][j] > work_matrix[j] {
                        can_finish = false;
                        break;
                    }
                }
                if can_finish {
                    // this thread can finish
                    found = true;
                    finish_matrix[i] = true;
                    for j in 0..process_inner.mutex_list.len() {
                        if allocation_matrix[i][j] {
                            work_matrix[j] = true; // release the mutex
                        }
                    }
                }
            }
            if !found {
                deadlock_detected = true;
                break;
            }
        }
        if deadlock_detected {
            drop(process_inner);
            drop(process);
            return -0xdead;
        }
    }
    // 看 mutex 是否是 locked

    if !mutex.is_locked() {
        // update allocation matrix
        let allocation_matrix = &mut process_inner.allocation_matrix;
        if allocation_matrix[cur_tid].len() <= mutex_id {
            // extend the allocation matrix for current thread
            for _ in allocation_matrix[cur_tid].len()..mutex_id + 1 {
                allocation_matrix[cur_tid].push(false);
            }
        }
        allocation_matrix[cur_tid][mutex_id] = true;
        let avail_mutexes = &mut process_inner.avail_mutexes;
        if avail_mutexes.len() <= mutex_id {
            // extend the avail mutexes for current thread
            for _ in avail_mutexes.len()..mutex_id + 1 {
                avail_mutexes.push(false);
            }
        }
        avail_mutexes[mutex_id] = false; // mark the mutex as unavailable
    } else {
        // update need matrix
        let need_matrix = &mut process_inner.need_matrix;
        if need_matrix[cur_tid].len() <= mutex_id {
            // extend the need matrix for current thread
            for _ in need_matrix[cur_tid].len()..mutex_id + 1 {
                need_matrix[cur_tid].push(false);
            }
        }
        need_matrix[cur_tid][mutex_id] = true;
    }

    drop(process_inner);
    drop(process);
    mutex.lock();
    let process_1 = current_process();
    let process_inner_1 = process_1.inner_exclusive_access();
    drop(process_inner_1);
    drop(process_1);
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
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    drop(process_inner);
    drop(process);
    mutex.unlock();
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
        let deadlock_detection_enabled = process_inner.deadlock_detection_enabled;
        if deadlock_detection_enabled {
            process_inner.avail_semaphores[id] = res_count;
        }
        id
    } else {
        process_inner
            .semaphore_list
            .push(Some(Arc::new(Semaphore::new(res_count))));
        let deadlock_detection_enabled = process_inner.deadlock_detection_enabled;
        if deadlock_detection_enabled {
            process_inner.avail_semaphores.push(res_count);
        }
        process_inner.semaphore_list.len() - 1
    };
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
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let sem = Arc::clone(process_inner.semaphore_list[sem_id].as_ref().unwrap());
    // append the length
    while process_inner.avail_semaphores.len() <= sem_id {
        process_inner.avail_semaphores.push(0);
    }
    process_inner.avail_semaphores[sem_id] += 1; // mark the semaphore as available
    // update allocation matrix
    let cur_tid = current_task()
        .unwrap()
        .inner_exclusive_access()
        .res
        .as_ref()
        .unwrap()
        .tid;
    let allocation_matrix = &mut process_inner.allocation_matrix_semaphore;
    if allocation_matrix[cur_tid].len() <= sem_id {
        // extend the allocation matrix for current thread
        for _ in allocation_matrix[cur_tid].len()..sem_id + 1 {
            allocation_matrix[cur_tid].push(0);
        }
    }
    allocation_matrix[cur_tid][sem_id] -= 1; // reduce the amount of the semaphore as allocated held
    drop(process_inner);
    sem.up();
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
    let process = current_process();
    let mut process_inner = process.inner_exclusive_access();
    let sem = Arc::clone(process_inner.semaphore_list[sem_id].as_ref().unwrap());
    // get detection
    let cur_tid = current_task()
        .unwrap()
        .inner_exclusive_access()
        .res
        .as_ref()
        .unwrap()
        .tid;
    if process_inner.deadlock_detection_enabled {
        // check for deadlock
        // println!("sys_semaphore_down: deadlock detection enabled");
        // println!("checking deadlock for tid: {}, sem_id: {}", cur_tid, sem_id);
        let sem_cnt = process_inner.semaphore_list.len();
        // update need matrix
        let mut need_matrix = process_inner.need_matrix_semaphore.clone();
        for line in need_matrix.iter_mut() {
            if line.len() < sem_cnt {
                // extend the need matrix for current thread
                for _ in line.len()..sem_cnt {
                    line.push(0);
                }
            }
        }
        // update the need with current need
        need_matrix[cur_tid][sem_id] += 1;
        let mut allocation_matrix = process_inner.allocation_matrix_semaphore.clone();
        for line in allocation_matrix.iter_mut() {
            if line.len() < sem_cnt {
                // extend the allocation matrix for current thread
                for _ in line.len()..sem_cnt {
                    line.push(0);
                }
            }
        }
        // println!("sys_semaphore_down: allocation matrix: {:?}", allocation_matrix);
        // println!("sys_semaphore_down: need matrix: {:?}", need_matrix);

        let mut deadlock_detected = false;
        let mut work_matrix = process_inner.avail_semaphores.clone();
        // println!("sys_semaphore_down: work matrix: {:?}", work_matrix);
        let tasks_num = process_inner.thread_count();
        let mut finish_matrix = Vec::new();
        let thread_cnt = process_inner.thread_count();
        for _ in 0..thread_cnt {
            finish_matrix.push(false);
        }
        // start detection
        while finish_matrix.iter().any(|&x| !x) {
            let mut found = false;
            for i in 0..tasks_num {
                if finish_matrix[i] {
                    continue;
                }
                // check if this thread can finish
                let mut can_finish = true;
                for j in 0..process_inner.semaphore_list.len() {
                    if need_matrix[i][j] > work_matrix[j] {
                        can_finish = false;
                        break;
                    }
                }
                if can_finish {
                    // this thread can finish
                    found = true;
                    finish_matrix[i] = true;
                    for j in 0..process_inner.semaphore_list.len() {
                        if allocation_matrix[i][j] > 0 {
                            work_matrix[j] += allocation_matrix[i][j]; // release the semaphore
                        }
                    }
                }
            }
            if !found {
                deadlock_detected = true;
                println!("[ERROR]: Deadlock detected for tid: {}, sem_id: {}", cur_tid, sem_id);
                break;
            }
        }
        if deadlock_detected {
            drop(process_inner);
            drop(process);
            return -0xdead;
        }
    }
    if sem.get_count() >= 1{
        // allocate it
        let allocation_matrix = &mut process_inner.allocation_matrix_semaphore;
        if allocation_matrix[cur_tid].len() <= sem_id {
            // extend the allocation matrix for current thread
            for _ in allocation_matrix[cur_tid].len()..sem_id + 1 {
                allocation_matrix[cur_tid].push(0);
            }
        }
        allocation_matrix[cur_tid][sem_id] += 1;
        let avail_semaphores = &mut process_inner.avail_semaphores;
        if avail_semaphores.len() <= sem_id {
            // extend the avail semaphores for current thread
            for _ in avail_semaphores.len()..sem_id + 1 {
                avail_semaphores.push(0);
            }
        }
        avail_semaphores[sem_id] -= 1; // mark the semaphore as unavailable
    }else{
        let need_matrix = &mut process_inner.need_matrix_semaphore;
        if need_matrix[cur_tid].len() <= sem_id {
            // extend the need matrix for current thread
            for _ in need_matrix[cur_tid].len()..sem_id + 1 {
                need_matrix[cur_tid].push(0);
            }
        }
        need_matrix[cur_tid][sem_id] += 1; // mark the semaphore as needed
    }

    // debug print
    // if process_inner.deadlock_detection_enabled {
    //     println!("after updating...");
    //     println!("sys_semaphore_down: allocation matrix: {:?}", process_inner.allocation_matrix_semaphore);
    //     println!("sys_semaphore_down: need matrix: {:?}", process_inner.need_matrix_semaphore);
    //     println!("sys_semaphore_down: avail semaphores: {:?}", process_inner.avail_semaphores);
    //     println!("after updating print end");
    // }
    drop(process_inner);
    sem.down();
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
    let process = current_process();
    let process_inner = process.inner_exclusive_access();
    let condvar = Arc::clone(process_inner.condvar_list[condvar_id].as_ref().unwrap());
    let mutex = Arc::clone(process_inner.mutex_list[mutex_id].as_ref().unwrap());
    drop(process_inner);
    condvar.wait(mutex);
    0
}
/// enable deadlock detection syscall
/// YOUR JOB: Implement deadlock detection, but might not all in this syscall
pub fn sys_enable_deadlock_detect(_enabled: usize) -> isize {
    if _enabled == 1 {
        // Enable deadlock detection for current process
        let process = current_process();
        let mut process_inner = process.inner_exclusive_access();
        process_inner.deadlock_detection_enabled = true;
        return 0;
    } else if _enabled == 0 {
        // Disable deadlock detection for current process
        let process = current_process();
        let mut process_inner = process.inner_exclusive_access();
        process_inner.deadlock_detection_enabled = false;
        return 0;
    }
    // Invalid argument
    -1
}
