/*stride 调度算法
ch3 中我们实现的调度算法十分简单。现在我们要为我们的 os 实现一种带优先级的调度算法：stride 调度算法。

算法描述如下:

(1) 为每个进程设置一个当前 stride，表示该进程当前已经运行的“长度”。另外设置其对应的 pass 值（只与进程的优先权有关系），表示对应进程在调度后，stride 需要进行的累加值。

每次需要调度时，从当前 runnable 态的进程中选择 stride 最小的进程调度。对于获得调度的进程 P，将对应的 stride 加上其对应的步长 pass。

一个时间片后，回到上一步骤，重新调度当前 stride 最小的进程。

可以证明，如果令 P.pass = BigStride / P.priority 其中 P.priority 表示进程的优先权（大于 1），而 BigStride 表示一个预先定义的大常数，则该调度方案为每个进程分配的时间将与其优先级成正比。证明过程我们在这里略去，有兴趣的同学可以在网上查找相关资料。

其他实验细节：

stride 调度要求进程优先级 
，所以设定进程优先级 
 会导致错误。

进程初始 stride 设置为 0 即可。

进程初始优先级设置为 16。

为了实现该调度算法，内核还要增加 set_prio 系统调用

// syscall ID：140
// 设置当前进程优先级为 prio
// 参数：prio 进程优先级，要求 prio >= 2
// 返回值：如果输入合法则返回 prio，否则返回 -1
fn sys_set_priority(prio: isize) -> isize;
提示

你可以在TCB加入新的字段来支持优先级等。

为了减少整数除的误差，BIG_STRIDE 一般需要很大，但为了不至于发生反转现象（详见问答作业），或许选择一个适中的数即可，当然能进行溢出处理就更好了。

stride 算法要找到 stride 最小的进程，使用优先级队列是效率不错的办法，但是我们的实验测例很简单，所以效率完全不是问题。事实上，很推荐使用暴力扫一遍的办法找最小值。

注意设置进程的初始优先级。
*/

/*
 * In this file, you have a data structure to manage the priority of processes. You can modify the data structure of TaskControlBlock to support priority scheduling, and implement the sys_set_priority system call.
 */

/// The priority info of a process, which is used in stride scheduling
pub const BIG_STRIDE: usize = 1 << 16;

/// The priority info of a process, which is used in stride scheduling
pub struct PriorityInfo {
    /// The priority of the process, which is set by sys_set_priority system call. The initial value is 16.
    pub priority: usize,
    /// The stride of the process, which is updated when the process is scheduled. The initial value is 0.
    pub stride: usize, 
    /// The pass of the process, which is calculated by BIG_STRIDE / priority. The initial value is BIG_STRIDE / 16.
    pub pass: usize,
}

/// The priority info of a process, which is used in stride scheduling
impl PriorityInfo {
    /// Create a new PriorityInfo with the given priority
    pub fn new(priority: usize) -> Self {
        Self {
            priority,
            stride: 0,
            pass: BIG_STRIDE / priority,
        }
    }

    /// Update the stride of the process when it is scheduled
    pub fn update_stride(&mut self) {
        self.stride += self.pass;
    }

    /// Set the priority of the process and update the pass accordingly
    pub fn set_priority(&mut self, priority: usize) {
        self.priority = priority;
        self.pass = BIG_STRIDE / priority;
    }

    /// Get the priority of the process
    pub fn get_priority(&self) -> usize {
        self.priority
    }

    /// Get the stride of the process
    pub fn get_stride(&self) -> usize {
        self.stride
    }
}

