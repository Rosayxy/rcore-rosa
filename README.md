# rCore-Tutorial-Code-2025S

### Code
- [Soure Code of labs for 2025S](https://github.com/LearningOS/rCore-Tutorial-Code-2025S)
### Documents

- Concise Manual: [rCore-Tutorial-Guide-2025S](https://LearningOS.github.io/rCore-Tutorial-Guide-2025S/)

- Detail Book [rCore-Tutorial-Book-v3](https://rcore-os.github.io/rCore-Tutorial-Book-v3/)


### OS API docs of rCore Tutorial Code 2025S
- [OS API docs of ch1](https://learningos.github.io/rCore-Tutorial-Code-2025S/ch1/os/index.html)
  AND [OS API docs of ch2](https://learningos.github.io/rCore-Tutorial-Code-2025S/ch2/os/index.html)
- [OS API docs of ch3](https://learningos.github.io/rCore-Tutorial-Code-2025S/ch3/os/index.html)
  AND [OS API docs of ch4](https://learningos.github.io/rCore-Tutorial-Code-2025S/ch4/os/index.html)
- [OS API docs of ch5](https://learningos.github.io/rCore-Tutorial-Code-2025S/ch5/os/index.html)
  AND [OS API docs of ch6](https://learningos.github.io/rCore-Tutorial-Code-2025S/ch6/os/index.html)
- [OS API docs of ch7](https://learningos.github.io/rCore-Tutorial-Code-2025S/ch7/os/index.html)
  AND [OS API docs of ch8](https://learningos.github.io/rCore-Tutorial-Code-2025S/ch8/os/index.html)
- [OS API docs of ch9](https://learningos.github.io/rCore-Tutorial-Code-2025S/ch9/os/index.html)

### Related Resources
- [Learning Resource](https://github.com/LearningOS/rust-based-os-comp2022/blob/main/relatedinfo.md)


### Build & Run

```bash
# setup build&run environment first
git clone https://github.com/LearningOS/rCore-Tutorial-Code-2025S.git
cd rCore-Tutorial-Code-*2025S
git clone https://github.com/LearningOS/rCore-Tutorial-Test-2025S.git user
cd os
git checkout ch$ID
# run OS in ch$ID
make run
```
Notice: $ID is from [1-9]

### Grading

```bash
# setup build&run environment first
git clone https://github.com/LearningOS/rCore-Tutorial-Code-2025S.git
cd rCore-Tutorial-Code-2025S
rm -rf ci-user
git clone https://github.com/LearningOS/rCore-Tutorial-Checker-2025S.git ci-user
git clone https://github.com/LearningOS/rCore-Tutorial-Test-2025S.git ci-user/user
git checkout ch$ID
# check&grade OS in ch$ID with more tests
cd ci-user && make test CHAPTER=$ID
```
Notice: $ID is from [3,4,5,6,8]


## notes
### 多道程序放置和加载
user/build.py 为每个程序定制各自的起始地址，用 -Clink-args=-Ttext=xxxx 选项指定链接时 .text 段的地址为 0x80400000 + app_id * 0x20000    
在原先的里面有 AppManager，负责循环加载一个程序进内存然后运行    
我们这里分为 loader 和 executor，loader 会在 init 的时候按顺序把所有的程序加载入内存    

### 任务切换：
在上一章中，我们先加载并运行第一个程序，当该程序返回时，会调用 sys_exit，sys_exit 之后会自然调用 run_next_app，如果程序发生 illegal instruction/pagefault 也会经过 trap_handler 再 run_next_app，从而实现按批执行程序    

在这里，我们应对的情况更加复杂，应用在运行中会主动或者被动的交出 cpu 的使用权，然后内核可以选择另一个程序继续执行    
它的特点在于：不涉及特权级切换，部分由 compiler 完成 && 对应用透明    

Task: 任务切换的基本单位，它的组成如下     
```rs
pub struct TaskControlBlock {
    /// The task status in it's lifecycle
    pub task_status: TaskStatus,
    /// The task context
    pub task_cx: TaskContext,
}
pub enum TaskStatus {
    /// uninitialized
    UnInit,
    /// ready to run
    Ready,
    /// running
    Running,
    /// exited
    Exited,
}
pub struct TaskContext {
    /// Ret position after task switching
    ra: usize,
    /// Stack pointer
    sp: usize,
    /// s0-11 register, callee saved
    s: [usize; 12],
}

```

在 函数 `pub fn __switch(current_task_cx_ptr: *mut TaskContext, next_task_cx_ptr: *const TaskContext);` 中，会保存目前的 TaskContext，然后切换next_task_cx 执行    

其中，“应用在运行中会主动交出 cpu 使用权” 是调用 sys_yield/sys_exit (yield 参考计系概)    
sys_yield: 目前的 status running -> ready，然后 run_next_task    

sys_exit: status running -> exited，然后 run_next_task    

run_next_task: 先找到比当前 index 大的 index 最小的 ready 的 task（或者比当前 index 小的 index 最小的 ready task），然后 mark it as running，再 __switch(current_task_cx_ptr, next_task_cx_ptr);   
初始化：
```rs
        for (i, task) in tasks.iter_mut().enumerate() {
            task.task_cx = TaskContext::goto_restore(init_app_cx(i));
            task.task_status = TaskStatus::Ready;
        }
```

然后执行 tasks[0]    

### 分时多任务系统
以时间片为应用连续执行时长的度量单位，采用时间片轮转系统对应用进行调度    
时钟中断：triggered when mtime > mtimecmp    
接口 `set_next_trigger` 可以在 10 ms 之后触发一个 S mode 时钟中断     

### 嵌套中断
Trap 进入某个特权级之后，在 Trap 处理的过程中同特权级的中断都会被屏蔽   
如果不手动设置 sstatus csr，则不会出现嵌套中断     

抢占式调度：改 Trap_handler，当触发 S mode 时间中断的时候会重新设置计时器，然后 suspend_current_and_run_next    
