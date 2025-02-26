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
实现虚存     

### SV39
satp 寄存器会 enable 虚拟地址和 mmu mapping    
mode 设置为 8 时，SV39 分页机制被启用，所有 S/U 特权级的访存被视为一个 39 位的虚拟地址     
地址格式和组成：看计组，virtual addr 39 bits physical addr 56 bits     

### 地址空间抽象
```rs
pub struct MapArea {
    vpn_range: VPNRange,
    data_frames: BTreeMap<VirtPageNum, FrameTracker>,
    map_type: MapType,
    map_perm: MapPermission,
}

```
描述一段虚拟空间    
MapType 分别有 Identical 和 Framed，分别表示恒等映射（启用多级页表之后仍能够访问一个特定的物理地址指向的物理内存），Frame 把每个虚拟页面映射到新的物理栈帧     

frame allocator    
每个 Frame 表示一个物理页帧     
对于这里的 StackFrameAllocator，
```rs
pub struct StackFrameAllocator {
    current: usize,
    end: usize,
    recycled: Vec<usize>,
}

```
物理页号区间 `[current,end)` 此前均从未被分配出去过，而向量 recycled 以后入先出的方式保存了被回收的物理页号     

内核和应用的地址空间：这块看文档吧，感觉讲的挺清楚的     

基于地址空间的分时多任务：     
建立并开启基于分页模式的虚拟地址空间：    
sbi 初始化之后，cpu 跳转到内核入口点并在 S mode 上执行，开启分页模式之后，内核的代码在访存的时候只能看到内核地址空间，此时每次访存将被视为一个虚拟地址且需要通过 MMU 基于内核地址空间的多级页表的地址转换。是否使用分页模式的转换发生在内核初始化期间     

那个 page table token 是会按照 satp CSR 格式要求 构造一个无符号 64 位无符号整数，使得其分页模式为 SV39 ，且将当前多级页表的根节点所在的物理页号填充进去     
在 activate 中，我们将这个值写入当前 CPU 的 satp CSR ，从这一刻开始 SV39 分页模式就被启用了    

跳板：无论是内核还是用户的地址空间，最高的虚拟页面都是一个跳板，用户地址空间的次高虚拟页面是来设置存放应用 Trap 的上下文     
为什么需要跳板：应用 Trap 到内核的时候，sscratch 指出了内核栈的栈顶，用一条指令可以由用户栈切换到内核栈，然后直接将 Trap 上下文压入内核栈栈顶。当 Trap 处理完毕返回用户态的时候，将 Trap 上下文中的内容恢复到寄存器上，最后将保存着应用用户栈顶的 sscratch 与 sp 进行交换，也就从内核栈切换回了用户栈    

使能了分页机制之后，我们需要在这个过程同时完成地址空间的切换，当 __alltraps 保存 Trap 上下文的时候，我们必须通过修改 satp 从应用地址空间切换到内核地址空间，__restore 同理，地址空间的切换不能影响指令的连续执行，这就要求应用和内核地址空间在切换地址空间指令附近是平滑的，所以才有了 Trampoline      

在之前我们看到了 TrapContext, 这里我们在 Trap 上下文中包含更多内容，包括 kernel_satp，kernel_sp, trap_handler     

trampoline: 看文档吧    

加载和执行应用程序：   
扩展应用控制块：多了 memory_set，trap_cx_ppn，base_size 三个字段     
因为 Trap 上下文不在内核地址空间，所以调用 current_trap_cx 来获取当前应用的 Trap 上下文的可变引用    

sys_write: 需要手动查页表才能知道，哪些数据被放置在哪些物理页帧上并进行访问     
page_table 里面的 translated_byte_buffer 可以以向量的形式返回一组可以在内核空间中直接访问的字节数组切片，可能在大作业中需要用到     

作业：如何判断地址用户不可读：