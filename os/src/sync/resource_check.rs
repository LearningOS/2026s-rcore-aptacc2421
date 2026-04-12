/*
死锁检测
目前的 mutex 和 semaphore 相关的系统调用不会分析资源的依赖情况，用户程序可能出现死锁。 我们希望在系统中加入死锁检测机制，当发现可能发生死锁时拒绝对应的资源获取请求。 一种检测死锁的算法如下：

定义如下三个数据结构：

可利用资源向量 Available ：含有 m 个元素的一维数组，每个元素代表可利用的某一类资源的数目， 其初值是该类资源的全部可用数目，其值随该类资源的分配和回收而动态地改变。 Available[j] = k，表示第 j 类资源的可用数量为 k。

分配矩阵 Allocation：n * m 矩阵，表示每类资源已分配给每个线程的资源数。 Allocation[i,j] = g，则表示线程 i 当前己分得第 j 类资源的数量为 g。

需求矩阵 Need：n * m 的矩阵，表示每个线程还需要的各类资源数量。 Need[i,j] = d，则表示线程 i 还需要第 j 类资源的数量为 d 。

算法运行过程如下：

设置两个向量: 工作向量 Work，表示操作系统可提供给线程继续运行所需的各类资源数目，它含有 m 个元素。初始时，Work = Available ；结束向量 Finish，表示系统是否有足够的资源分配给线程， 使之运行完成。初始时 Finish[0..n-1] = false，表示所有线程都没结束；当有足够资源分配给线程时， 设置 Finish[i] = true。

从线程集合中找到一个能满足下述条件的线程

1Finish[i] == false;
2Need[i,j] <= Work[j];
若找到，执行步骤 3，否则执行步骤 4。

当线程 thr[i] 获得资源后，可顺利执行，直至完成，并释放出分配给它的资源，故应执行:

1Work[j] = Work[j] + Allocation[i, j];
2Finish[i] = true;
跳转回步骤2

如果 Finish[0..n-1] 都为 true，则表示系统处于安全状态；否则表示系统处于不安全状态，即出现死锁。

出于兼容性和灵活性考虑，我们允许进程按需开启或关闭死锁检测功能。为此我们将实现一个新的系统调用： sys_enable_deadlock_detect 。
*/
use alloc::vec;
use alloc::vec::Vec;
use alloc::boxed::Box;

/// trait get single resource check info
pub trait SingleResourceInfo {
    fn get_available(&self) -> usize;
}

pub struct ResourceCheck {
    /// 可利用资源向量 Available ：含有 m 个元素的一维数组，每个元素代表可利用的某一类资源的数目， 其初值是该类资源的全部可用数目，其值随该类资源的分配和回收而动态地改变。 Available[j] = k，表示第 j 类资源的可用数量为 k。
    available: Vec<usize>,
    /// 分配矩阵 Allocation：n * m 矩阵，表示每类资源已分配给每个线程的资源数。 Allocation[i,j] = g，则表示线程 i 当前己分得第 j 类资源的数量为 g。
    allocation: Vec<Vec<usize>>,
    /// 需求矩阵 Need：n * m 的矩阵，表示每个线程还需要的各类资源数量。 Need[i,j] = d，则表示线程 i 还需要第 j 类资源的数量为 d 。
    need: Vec<Vec<usize>>,
    /// 结束向量 Finish，表示系统是否有足够的资源分配给线程， 使之运行完成。初始时 Finish[0..n-1] = false，表示所有线程都没结束；当有足够资源分配给线程时， 设置 Finish[i] = true。
    finish: Vec<bool>,
}

impl ResourceCheck {
    fn new(num_threads: usize, num_resources: usize, total_resources: Vec<Box<dyn SingleResourceInfo>>) -> Self {
        Self {
            available: total_resources.iter().map(|r| r.get_available()).collect(),
            allocation: vec![vec![0; num_resources]; num_threads],
            need: vec![vec![0; num_resources]; num_threads],
            finish: vec![false; num_threads],
        }  
    }

    fn request_resource(&mut self, thread_id: usize, request_id: usize, request_amount: usize) -> bool {
        // 检查请求是否合法
        if request_amount == 0 {
            return true; // 请求为0，直接通过
        }
        if request_id >= self.available.len() || thread_id >= self.allocation.len() {
            return false;
        }
        // 检查请求是否超过需求
        if self.need[thread_id][request_id] < request_amount {
            return false;
        }
        // 检查请求是否超过可用资源
        if self.available[request_id] < request_amount {
            return false;
        }
        // 模拟分配资源
        self.available[request_id] -= request_amount;
        self.allocation[thread_id][request_id] += request_amount;
        self.need[thread_id][request_id] -= request_amount;

        // 检测死锁
        self.detect_deadlock()
    }

    fn detect_deadlock(&mut self) -> bool {
        // 初始化工作向量和结束向量
        let mut work = self.available.clone();
        self.finish.iter_mut().for_each(|f| *f = false);

        loop {
            let mut found = false;
            for i in 0..self.allocation.len() {
                if !self.finish[i] && self.need[i].iter().zip(&work).all(|(n, w)| *n <= *w) {
                    // 线程 i 可以完成
                    for j in 0..work.len() {
                        work[j] += self.allocation[i][j];
                    }
                    self.finish[i] = true;
                    found = true;
                }
            }
            if !found {
                break; // 没有找到可完成的线程，退出循环
            }
        }

        // 检查是否所有线程都完成
        self.finish.iter().all(|&f| f)
    }

    fn release_resource(&mut self, thread_id: usize, release_id: usize, release_amount: usize) {
        // 模拟释放资源
        if release_id < self.available.len() && thread_id < self.allocation.len() {
            self.available[release_id] += release_amount;
            self.allocation[thread_id][release_id] -= release_amount;
            self.need[thread_id][release_id] += release_amount;
        }
    }
}
