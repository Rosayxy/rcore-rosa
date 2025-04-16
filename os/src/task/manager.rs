//!Implementation of [`TaskManager`]
use super::{TaskControlBlock, TaskStatus};
use crate::config::BIG_STRIDE_NUM;
use crate::sync::UPSafeCell;

use alloc::sync::Arc;
use alloc::vec::Vec;
use lazy_static::*;
///A array of `TaskControlBlock` that is thread-safe
pub struct TaskManager {
    ready_queue: Vec<Arc<TaskControlBlock>>,
}

/// A simple FIFO scheduler.
impl TaskManager {
    ///Creat an empty TaskManager
    pub fn new() -> Self {
        Self {
            ready_queue: Vec::new(),
        }
    }
    /// Add process back to ready queue
    pub fn add(&mut self, task: Arc<TaskControlBlock>) {
        self.ready_queue.push(task);
    }
    /// Take a process out of the ready queue
    pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
        // enumerate ready queue to get the one with the smallest stride
        if self.ready_queue.is_empty() {
            return None;
        }
        let mut min_stride = BIG_STRIDE_NUM;
        let mut min_stride_task_idx = 0;
        for i in 0..self.ready_queue.len() {
            let task = self.ready_queue[i].inner_exclusive_access();
            if task.task_status == TaskStatus::Ready
                && task.stride < min_stride
                && task.stride != 0
            {
                min_stride_task_idx = i;
                min_stride = task.stride;
            }
        }
        // take the task with the smallest stride
        let task = self.ready_queue.remove(min_stride_task_idx);
        // set the stride to add the pass
        let pass = task.inner_exclusive_access().pass;
        task.inner_exclusive_access().stride += pass;
        Some(task)
    }
}

lazy_static! {
    /// TASK_MANAGER instance through lazy_static!
    pub static ref TASK_MANAGER: UPSafeCell<TaskManager> =
        unsafe { UPSafeCell::new(TaskManager::new()) };
}

/// Add process to ready queue
pub fn add_task(task: Arc<TaskControlBlock>) {
    //trace!("kernel: TaskManager::add_task");
    TASK_MANAGER.exclusive_access().add(task);
}

/// Take a process out of the ready queue
pub fn fetch_task() -> Option<Arc<TaskControlBlock>> {
    //trace!("kernel: TaskManager::fetch_task");
    TASK_MANAGER.exclusive_access().fetch()
}
