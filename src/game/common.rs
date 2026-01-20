//! 游戏自动化公共模块
//!
//! 包含所有版本共用的函数

use anyhow::{Context, Result};
use std::thread;
use std::time::Duration;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowW, MoveWindow, SetWindowPos, HWND_TOPMOST, SWP_SHOWWINDOW,
};

use crate::keys::{
    click_at, left_click_legacy, move_to, press_key, tap_key, VK_4, VK_5, VK_A, VK_D, VK_G, VK_N,
    VK_SPACE,
};
use crate::ocr::{clear_frame_cache, find_text_contains, ocr_screen};
use crate::stop_flag::should_stop;

/// 移动基础值
pub const MOVE_VALUE: i32 = 22;

/// 是否调试模式 - 开启后会打印 OCR 结果
pub const IS_DEBUG: bool = true;

/// 将 Rust 字符串转换为 Windows 宽字符串
pub fn to_wide_string(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// 查找游戏窗口
pub fn find_game_window() -> Option<HWND> {
    // 注意：窗口标题末尾有两个空格
    let title = to_wide_string("逆战：未来  ");
    unsafe {
        match FindWindowW(None, windows::core::PCWSTR(title.as_ptr())) {
            Ok(hwnd) if hwnd.0 != std::ptr::null_mut() => Some(hwnd),
            _ => None,
        }
    }
}

/// 设置窗口位置和大小
pub fn setup_window(hwnd: HWND) -> Result<()> {
    unsafe {
        // 移动窗口到 (0, 0) 并调整大小为 1920x1080
        MoveWindow(hwnd, 0, 0, 1920, 1080, true)?;

        // 设置窗口置顶
        SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 1920, 1080, SWP_SHOWWINDOW)?;
    }
    Ok(())
}

/// 开始游戏 - difficulty 参数指定要点击的难度文字（如 "困难"、"炼狱"、"普通"）
pub fn start_game_with_difficulty(difficulty: &str) -> Result<()> {
    println!("[startGame] 查找游戏窗口...");

    let hwnd = find_game_window().context("未找到游戏窗口 '逆战：未来'")?;
    println!("[startGame] 窗口已找到");

    // 设置窗口
    setup_window(hwnd)?;
    let results = ocr_screen(84, 230, 393, 61, false, IS_DEBUG)?;

    // 判断如果不是空间站，则停止
    if find_text_contains(&results, "空间站").is_none() {
        anyhow::bail!("当前不在空间站，无法开始游戏");
    }

    // OCR 识别屏幕
    let results = ocr_screen(1182, 0, 738, 1080, false, IS_DEBUG)?;

    for result in &results {
        if should_stop() {
            println!("[STOP] startGame: 检测到停止信号");
            return Ok(());
        }

        let (center_x, center_y) = result.center();

        // 检测指定难度
        if result.text.contains(difficulty) {
            println!("[startGame] 找到 '{}'，点击", difficulty);
            click_at(center_x, center_y);
            thread::sleep(Duration::from_millis(200));
        }

        // 检测"创建房间"
        if result.text.contains("创建房间") {
            println!("[startGame] 找到 '创建房间'，点击");
            click_at(1362, 875);
            thread::sleep(Duration::from_millis(200));
            click_at(1685, 930);
            thread::sleep(Duration::from_millis(200));
        }

        // 检测"开始"
        if result.text.contains("开始") {
            println!("[startGame] 找到 '开始'，点击");
            click_at(1685, 930);
            thread::sleep(Duration::from_millis(200));
        }
    }

    thread::sleep(Duration::from_secs(1));

    // 按空格跳过开场
    press_key(VK_SPACE, 2.0);
    thread::sleep(Duration::from_secs(5));

    // 循环等待游戏开始
    println!("[startGame] 等待游戏开始...");
    loop {
        if should_stop() {
            println!("[STOP] startGame: 检测到停止信号");
            break;
        }

        let results = ocr_screen(0, 0, 1920, 1080, false, IS_DEBUG)?;

        let found = results
            .iter()
            .any(|r| r.text.contains("怪物即将来袭") || r.text.contains("波次1"));

        if found {
            println!("[startGame] 找到游戏开始标志");
            break;
        }

        thread::sleep(Duration::from_secs(1));
    }

    Ok(())
}

/// 购买陷阱
pub fn buy_traps() -> Result<()> {
    if should_stop() {
        println!("[STOP] buy_traps: 检测到停止信号，跳过");
        return Ok(());
    }

    println!("[buy_traps] 打开商店");
    tap_key(VK_N);
    thread::sleep(Duration::from_secs(1));

    if should_stop() {
        tap_key(VK_N);
        return Ok(());
    }

    let results = ocr_screen(0, 0, 1920, 1080, false, IS_DEBUG)?;
    println!("[buy_traps] OCR 结果：");

    for result in &results {
        if should_stop() {
            tap_key(VK_N);
            return Ok(());
        }

        println!("  识别到: [{}] 坐标: {:?}", result.text, result.box_points);

        // 查找破坏者或自修复磁暴塔
        if result.text.contains("破坏者") || result.text.contains("自修复磁暴塔") {
            println!("[buy_traps] 找到 '{}'，购买", result.text);
            let (center_x, center_y) = result.center();

            // 先移动到位置（偏移 50 像素）
            move_to(center_x + 50, center_y + 50);
            thread::sleep(Duration::from_millis(200));
            // 多次点击确保购买成功
            left_click_legacy();
            thread::sleep(Duration::from_millis(200));
            left_click_legacy();
            thread::sleep(Duration::from_millis(200));
            left_click_legacy();
            thread::sleep(Duration::from_millis(500));
        }
    }

    // 关闭商店
    tap_key(VK_N);
    Ok(())
}

/// 等待游戏结束
pub fn wait_for_game_end() -> Result<()> {
    println!("[wait_for_game_end] 等待游戏结束...");

    loop {
        if should_stop() {
            println!("[STOP] wait_for_game_end: 检测到停止信号");
            break;
        }

        let results = ocr_screen(0, 0, 1920, 1080, false, IS_DEBUG)?;

        // 检测游戏结束
        let game_ended = results.iter().any(|r| {
            r.text.contains("开始") || r.text.contains("炼狱") || r.text.contains("训练基地")
        });

        if game_ended {
            println!("[wait_for_game_end] 游戏结束");
            break;
        }

        // 处理"返回游戏"弹窗
        if let Some(result) = find_text_contains(&results, "返回游戏") {
            let (x, y) = result.center();
            move_to(x + 50, y + 50);
            thread::sleep(Duration::from_millis(200));
            left_click_legacy();
            thread::sleep(Duration::from_millis(200));
            left_click_legacy();
            thread::sleep(Duration::from_millis(200));
            left_click_legacy();
            thread::sleep(Duration::from_millis(500));
            continue;
        }

        // 检测"赛季经验"并截图
        if find_text_contains(&results, "赛季经验").is_some() {
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            let filename = format!("game_end_{}.png", timestamp);

            if let Ok(img) = crate::screen::capture_fullscreen() {
                let _ = crate::screen::save_screenshot(&img, &filename);
            }
            break;
        }

        // 保持活动
        press_key(VK_D, 0.5);
        press_key(VK_A, 0.5);
        press_key(VK_4, 5.0);
        press_key(VK_5, 5.0);
        tap_key(VK_SPACE);
        tap_key(VK_G);

        println!("[wait_for_game_end] 等待中...");
    }

    Ok(())
}

/// 清空 OCR 缓存的包装函数
pub fn clear_cache() {
    clear_frame_cache();
}
