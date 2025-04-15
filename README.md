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
$ git clone https://github.com/LearningOS/rCore-Tutorial-Code-2025S.git
$ cd rCore-Tutorial-Code-2025S
$ git clone https://github.com/LearningOS/rCore-Tutorial-Test-2025S.git user
$ cd os
$ git checkout ch$ID
# run OS in ch$ID
$ make run
```
Notice: $ID is from [1-9]

### Grading

```bash
# setup build&run environment first
$ git clone https://github.com/LearningOS/rCore-Tutorial-Code-2025S.git
$ cd rCore-Tutorial-Code-2025S
$ git clone https://github.com/LearningOS/rCore-Tutorial-Checker-2025S.git ci-user
$ git clone https://github.com/LearningOS/rCore-Tutorial-Test-2025S.git ci-user/user
$ cd ci-user && make test CHAPTER=$ID
```
Notice: $ID is from [3,4,5,6,8]

## notes
与进程有关的重要系统调用：fork & execve(将当前进程的地址空间清空并加载一个特定的可执行文件，返回用户态后开始它的执行)    

waitpid: 当前进程等待一个子进程变成 zombie，然后回收其全部资源并收集其返回值      

为了获取用户输入，定义了 sys_read 的系统调用，调用方式 belike `read(STDIN, &mut c);`       

而 sys_read 如下   
```rs
pub fn sys_read(fd: usize, buffer: &mut [u8]) -> isize {
    syscall(
        SYSCALL_READ,
        [fd, buffer.as_mut_ptr() as usize, buffer.len()],
    )
}

```
需要提供 buffer 地址和长度     

基于应用名的应用链接/加载器：见 os/build.rs ，我们按顺序保存链接进来的每个应用的名字  

APP_NAMES 是全局可见的只读向量    

`get_app_date_by_name` `list_apps` 分别可以**按照应用的名字来查找获得应用的 ELF 数据**和**打印出所有可用应用的名字**       

PIDHandle： 进程标识符     

PidAllocator: 类似于 FrameAllocator，实现了 PidAllocator     

内核栈：将应用编号替换为进程标识符来决定每个进程内核栈在地址空间中的位置     

kernel stack 的位置由 pid 决定，是看 `kernel_stack_position` 函数      

kernel stack 用到了 RAII 的思想，实际保存它的物理页帧的生命周期被绑定到它下面，当 KernelStack 生命周期结束后，这些物理页帧也将会被编译器自动回收     

每个进程的信息保存在 process control block 里面，我们**魔改任务控制块（Task control block） 来直接承担进程控制块的功能**     

实现了 `TaskControlBlockInner` 提供的方法主要是对于它内部字段的快捷访问     

TaskControlBlock: 实现了 inner_exclusive_access, get_pid, new, exec, fork 等功能    

任务管理器：将任务管理器对于 CPU 的监控职能拆分到处理器管理结构 Processor 中去，任务管理器只负责管理所有任务     

具体来说，之前说的任务调度和 run tasks 都扔到了 Processor 里面，manager 的实现主要是维护 ready_queue，把进程从 ready_queue 里面塞进去或者拿出来    

### 过程
初始进程创建：加载 `ch5b_initproc` 然后 `add_task(INITPROC.clone())`     

我们用 `TaskControlBlock::new` 来创建一个进程控制块，需要传入 elf 文件的 data 为参数，而这个可以通过 loader 的 get_app_data_by_name 来查到    

suspend_current_and_run_next：主要在两个地方用到：`sys_yield` 和 `trap_handler`    
而他的实现现在长这样：把当前 task 的 status 改为 ready，然后把 task 加到 ready queue 里面，再回到 scheduling cycle     

fork 的实现：最为关键且困难的一点是**为子进程创建一个和父进程几乎完全相同的地址空间**，这里的实现看以下接口：   
`MapArea::from_another` 可以把从一个逻辑段复制得到一个虚拟地址区间、映射方式和权限控制均相同的逻辑段，但是新复制出的虚拟段还没被映射到物理页帧，所以 `data_frames` 字段为空      
`MemorySet::from_existing_user`(这个是主要的函数) 复制一个完全相同的地址空间    
然后就是我们遍历原地址空间中的所有逻辑段，将复制之后的逻辑段插入新的地址空间， 在插入的时候就已经实际分配了物理页帧了。接着我们遍历逻辑段中的每个虚拟页面，对应完成数据复制， 这只需要找出两个地址空间中的虚拟页面各被映射到哪个物理页帧，就可转化为将数据从物理内存中的一个位置复制到另一个位置，使用 copy_from_slice 即可轻松实现    

然后用 TaskControlBlock 的 fork 从父进程的进程控制块 fork 出来子进程的进程控制块    

exec 系统调用：个进程能够加载一个新的 ELF 可执行文件替换原有的应用地址空间并开始执行      
先是进程控制块的 exec：它会先解析出来 memory_set user_sp 和 entrypoint, 然后从 ELF 生成一个全新的地址空间并直接替换进来，再修改新的地址空间中的 Trap 上下文，将解析得到的应用入口点、用户栈位置以及一些内核的信息进行初始化      
然后是 sys_exec 的实现     

### 迁移 syscall
把 task_manager 里面 get_ranges,insert/unmap frame 这些迁到 processor 里面去   
然后是 sys_exec 的实现，先是调用 translated_str 找到要执行的应用名，并试图从应用加载器提供的 get_app_data_by_name 接口中获取对应的 ELF 数据，如果找到的话就调用 TaskControlBlock::exec 替换地址空间    

syscall 之后重新获取进程上下文：新的 trap_handler 需要在syscall 返回之后重新获取 cx，即是当前应用的 Trap 上下文     

sys_read 是用 sbi 提供的借口 "console_getchar" 实现的    

exit_current_and_run_next: "相比前面的章节， exit_current_and_run_next 带有一个退出码作为参数，这个退出码会在 exit_current_and_run_next 写入当前进程的进程控制块"     

父进程回收子进程资源：看 sys_waitpid 的实现    

编程作业：思考维护 sys_get_time sys_mmap sys_munmap 和之前的相比有哪些不同，直觉是获得进程信息的渠道不一样     

spawn 同时整合了 fork + exec 的实现，但是**不必像 fork 一样复制父进程的地址空间**    

stride 调度：TODO 写的时候施工吧    

可能大概看到了

spawn：先在 sys_fork 基础上魔改吧，sys_fork 是创建一个新的 TaskControlBlock 然后塞到 scheduler 里面，sys_exec 是把当前 task 的地址空间改了，所以整体式套 sys_fork 的框架然后在创建 new_task 的时候把地址空间给换了   
