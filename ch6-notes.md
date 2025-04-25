# ch6 notes
先把 sys_spawn 迁移过去再说！！    
已迁移   
改 open_file 或者 sys_open 函数，在 ROOT_INODE.find(name) 的时候特判 name   
````rs
#[repr(C)]
#[derive(Debug)]
pub struct Stat {
    /// 文件所在磁盘驱动器号，该实验中写死为 0 即可
    pub dev: u64,
    /// inode 文件所在 inode 编号
    pub ino: u64,
    /// 文件类型
    pub mode: StatMode,
    /// 硬链接数量，初始为1
    pub nlink: u32,
    /// 无需考虑，为了兼容性设计
    pad: [u64; 7],
}

/// StatMode 定义：
bitflags! {
    pub struct StatMode: u32 {
        const NULL  = 0;
        /// directory
        const DIR   = 0o040000;
        /// ordinary regular file
        const FILE  = 0o100000;
    }
}
````
想一下输入为 fd，怎么获取所有字段   
fd table 在 Task ControlBlockInner 里面，用 fd 去索引到一个 Option<Arc<dyn File + Send + Sync>>    

如何去用这个索引一个 OSInode：这个 Arc<dyn traits> 就是一个 OSInode    

看一下测例会用到哪些字段吧    
fstat 返回值，mode nlink   
ino 只用保证硬链接前后一致就行（    
看 block_id 和 block_offset 干啥的，dir 和 file 如何区分（试一下调 Inode 的 fs 函数）    
sys_linkat/sys_unlinkat 的实现只看 OS，来维护一个 linkname -> realname 的数组（在 OS 里面维护就行）    

同时在 OSInode 对于 File trait 的实现增加结构，具体来说就是一个通过 Inode 获取到 Stat 的函数，以及 OSInode 来维护 nlink 吧      

nlink 在每次 sys_linkat/sys_unlink/sys_open/sys_close 维护    

无语了，要把用户传入的 stat 指针和对应 fd 关联，每次更新的时候都保留一波，看错了，应该是别的问题   
不知道了 不想做   
