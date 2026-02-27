//! 后台监控模块
//!
//! 提供波次和金币的持续 OCR 监控。
//! 两个独立线程在后台运行，通过原子变量共享状态。

use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU32, Ordering};
use std::thread;
use std::time::Duration;

use crate::ocr::{ocr_screen_color_filter, ocr_screen_small};
use crate::stop_flag::should_stop;

// ===== 全局状态 =====

/// 当前波次（0 = 未开始，1 = 第一波）
static CURRENT_WAVE: AtomicU32 = AtomicU32::new(0);

/// 当前金币
static CURRENT_GOLD: AtomicI64 = AtomicI64::new(0);

/// 监控是否在运行
static MONITOR_RUNNING: AtomicBool = AtomicBool::new(false);

// ===== 配置 =====

/// 监控配置
#[derive(Debug, Clone)]
pub struct MonitorConfig {
    /// 波次 OCR 区域 (x, y, w, h)
    pub wave_region: (i32, i32, i32, i32),
    /// 金币 OCR 区域 (x, y, w, h)
    pub gold_region: (i32, i32, i32, i32),
    /// 波次检测间隔 (毫秒)
    pub wave_interval_ms: u64,
    /// 金币检测间隔 (毫秒)
    pub gold_interval_ms: u64,
    /// 金币文字颜色 (R, G, B)，用于颜色过滤
    pub gold_text_color: (u8, u8, u8),
    /// 金币颜色容差（RGB 欧氏距离）
    pub gold_color_tolerance: f64,
    /// 是否使用颜色过滤（false 则用 Otsu 二值化）
    pub gold_use_color_filter: bool,
}

impl Default for MonitorConfig {
    fn default() -> Self {
        Self {
            wave_region: (1841, 733, 172, 52),
            gold_region: (48, 56, 120, 22),
            wave_interval_ms: 500,
            gold_interval_ms: 300,
            gold_text_color: (0xd9, 0xe1, 0xe3), // #d9e1e3
            gold_color_tolerance: 35.0,
            gold_use_color_filter: true,
        }
    }
}

// ===== 公开 API =====

/// 读取当前波次
pub fn current_wave() -> u32 {
    CURRENT_WAVE.load(Ordering::Relaxed)
}

/// 读取当前金币
pub fn current_gold() -> i64 {
    CURRENT_GOLD.load(Ordering::Relaxed)
}

/// 重置监控状态
pub fn reset_monitors() {
    CURRENT_WAVE.store(0, Ordering::Relaxed);
    CURRENT_GOLD.store(0, Ordering::Relaxed);
}

/// 监控是否在运行
pub fn is_running() -> bool {
    MONITOR_RUNNING.load(Ordering::Relaxed)
}

/// 启动后台监控线程（波次 + 金币）
pub fn start_monitors(config: MonitorConfig) {
    if MONITOR_RUNNING.load(Ordering::Relaxed) {
        println!("[Monitor] 监控已在运行");
        return;
    }

    MONITOR_RUNNING.store(true, Ordering::Relaxed);
    println!("[Monitor] 启动后台监控");

    // 波次监控线程
    let wave_config = config.clone();
    thread::spawn(move || {
        wave_monitor_loop(wave_config);
    });

    // 金币监控线程
    thread::spawn(move || {
        gold_monitor_loop(config);
    });
}

/// 停止后台监控
pub fn stop_monitors() {
    MONITOR_RUNNING.store(false, Ordering::Relaxed);
    println!("[Monitor] 停止后台监控");
}

// ===== 内部实现 =====

/// 波次监控循环（直接 OCR 数字，和金币一样的逻辑）
fn wave_monitor_loop(config: MonitorConfig) {
    let (x, y, w, h) = config.wave_region;
    let interval = Duration::from_millis(config.wave_interval_ms);

    println!(
        "[Monitor:Wave] 启动 | 区域: ({},{},{},{}) | 间隔: {}ms",
        x, y, w, h, config.wave_interval_ms
    );

    while MONITOR_RUNNING.load(Ordering::Relaxed) && !should_stop() {
        if let Ok(results) = ocr_screen_small(x, y, w, h, 3, false) {
            for result in &results {
                if let Some(wave) = parse_wave_number(&result.text) {
                    let old_wave = CURRENT_WAVE.load(Ordering::Relaxed);
                    if wave != old_wave && wave > 0 {
                        CURRENT_WAVE.store(wave, Ordering::Relaxed);
                        println!("[Monitor:Wave] 波次: {} → {}", old_wave, wave);
                    }
                }
            }
        }

        thread::sleep(interval);
    }

    println!("[Monitor:Wave] 已停止");
}

/// 金币监控循环
fn gold_monitor_loop(config: MonitorConfig) {
    let (x, y, w, h) = config.gold_region;
    let interval = Duration::from_millis(config.gold_interval_ms);
    let use_color = config.gold_use_color_filter;
    let color = config.gold_text_color;
    let tolerance = config.gold_color_tolerance;

    println!(
        "[Monitor:Gold] 启动 | 区域: ({},{},{},{}) | 间隔: {}ms | 颜色过滤: {}",
        x, y, w, h, config.gold_interval_ms, use_color
    );

    while MONITOR_RUNNING.load(Ordering::Relaxed) && !should_stop() {
        let results = if use_color {
            ocr_screen_color_filter(x, y, w, h, 3, color, tolerance, false)
        } else {
            ocr_screen_small(x, y, w, h, 3, false)
        };

        if let Ok(results) = results {
            for result in &results {
                if let Some(gold) = parse_gold(&result.text) {
                    CURRENT_GOLD.store(gold, Ordering::Relaxed);
                }
            }
        }

        thread::sleep(interval);
    }

    println!("[Monitor:Gold] 已停止");
}

/// 从文字中提取波次数字（直接提取所有数字）
/// "02" → Some(2)
/// "10" → Some(10)
/// "波次3" → Some(3)
fn parse_wave_number(text: &str) -> Option<u32> {
    let num_str: String = text.chars().filter(|c| c.is_ascii_digit()).collect();
    if num_str.is_empty() {
        return None;
    }
    num_str.parse().ok()
}

/// 从文字中提取金币数
/// "$3,999,600" → Some(3999600)
/// "3.979,600" → Some(3979600)
fn parse_gold(text: &str) -> Option<i64> {
    // 去掉所有非数字字符
    let num_str: String = text.chars().filter(|c| c.is_ascii_digit()).collect();
    if num_str.is_empty() {
        return None;
    }

    num_str.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_wave_number() {
        assert_eq!(parse_wave_number("02"), Some(2));
        assert_eq!(parse_wave_number("10"), Some(10));
        assert_eq!(parse_wave_number("1"), Some(1));
        assert_eq!(parse_wave_number("波次3"), Some(3));
        assert_eq!(parse_wave_number("没有数字"), None);
    }

    #[test]
    fn test_parse_gold() {
        assert_eq!(parse_gold("$3,999,600"), Some(3999600));
        assert_eq!(parse_gold("3.979,600"), Some(3979600));
        assert_eq!(parse_gold("4000000"), Some(4000000));
        assert_eq!(parse_gold("没有数字"), None);
    }
}
