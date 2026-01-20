//! 线程安全的停止标志模块
//!
//! 使用 AtomicBool 实现全局停止信号

use std::sync::atomic::{AtomicBool, Ordering};

/// 全局停止标志
static STOP_REQUESTED: AtomicBool = AtomicBool::new(false);

/// 请求停止所有任务
pub fn request_stop() {
    STOP_REQUESTED.store(true, Ordering::SeqCst);
}

/// 检查是否应该停止
pub fn should_stop() -> bool {
    STOP_REQUESTED.load(Ordering::SeqCst)
}

/// 重置停止标志（用于重新启动）
pub fn reset_stop() {
    STOP_REQUESTED.store(false, Ordering::SeqCst);
}
