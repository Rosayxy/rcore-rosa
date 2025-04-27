# lab3 report
于新雨 计25 2022010841
## 实现功能
首先，把 ch4 的 syscall 迁移过来了   
sys_spawn 实现：   
- 首先根据路径名称获取 app 的 data   
- 然后由 data 创建 task_control_block, 根据 elf_data 初始化 TrapContext 等结构体，然后创建新 task_control_block 并且设置为当前 task_control_block 的 children   
- 最后把新的 task control block 加入到 scheduler    

然后实现 stride 算法   
- 在 task_control_block 里面加入到 stride, pass, priority 信息，然后用 set_prio 信息设置 priority 字段
- 改了 TaskManager 的 fetch 函数，让他遍历当前的 ready_queue 每次选取 stride 最小的 task 返回    

## 回答问题

1. 不会，因为在 p2 执行一个时间片后 stride += pass 然后发生上溢，stride 变为 4，所以还是调度 p2   

2. pass 的最大值为 BIG_STRIDE/2，在进程初始 stride 都为0的情况下，需要证明    
假设某一次调度后 `STRIDE_MAX – STRIDE_MIN <= BigStride / 2`，则下一次调度依旧有 `STRIDE_MAX – STRIDE_MIN <= BigStride / 2`    

设之前 stride 最大的进程为 A 最小的进程为 B，则该次调度 B 执行    

则 B <= B_new <= B + BigStride/B_Prio <= B + BigStride / 2   

若 B_new >= A，则有 B <= A <= B_new <= B + BigStride/B_Prio    

若 B_new <= A，则有 A - BigStride / 2 <= B <= B_new <= A    

初始条件满足 STRIDE_MAX – STRIDE_MIN <= BigStride / 2 得证    

3. 
```rs
use core::cmp::Ordering;

struct Stride(u64);

impl PartialOrd for Stride {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // considering the possibility of overflow, get the comparison
        // use the condition of 初始条件满足 STRIDE_MAX – STRIDE_MIN <= BigStride / 2 得证    
        let BIG_STRIDE: u8 = 0xff;
        if self.0 == other.0 {
            return Some(Ordering::Equal);
        }
        
        if self.0 > other.0 {
            if self.0 - other.0 > BIG_STRIDE / 2 {
                return Some(Ordering::Less);
            } else {
                return Some(Ordering::Greater);
            }
        }

        if other.0 - self.0 > BIG_STRIDE / 2 {
            return Some(Ordering::Greater);
        } else {
            return Some(Ordering::Less);
        }
    }
}

impl PartialEq for Stride {
    fn eq(&self, other: &Self) -> bool {
        false
    }
}
```
## honor code
在完成本次实验的过程（含此前学习的过程）中，我曾分别与 以下各位 就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：

-

此外，我也参考了 以下资料 ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：

-

3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。

4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。