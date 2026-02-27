//! 游戏自动化公共模块
//!
//! 包含所有版本共用的函数

use anyhow::{Context, Result};
use std::thread;
use std::time::Duration;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowW, MoveWindow,
};

use crate::input::{
    click_at, get_vk_code, left_click, move_to, press_key, send_relative, tap_key, VK_5, VK_6,
    VK_G, VK_N, VK_SPACE,
};
use crate::monitor;
use crate::ocr::{clear_frame_cache, find_text_contains, ocr_screen};
use crate::screen::{full_screen_region, get_screen_resolution, scale_region, scale_x, scale_y};
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

/// 设置窗口位置和大小（自动适配屏幕分辨率）
pub fn setup_window(hwnd: HWND) -> Result<()> {
    let (w, h) = get_screen_resolution();
    unsafe {
        MoveWindow(hwnd, 0, 0, w as i32, h as i32, true)?;
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
    let (rx, ry, rw, rh) = scale_region(84, 230, 393, 61);
    let results = ocr_screen(rx, ry, rw, rh, false, IS_DEBUG)?;

    // 判断如果不是空间站，则停止
    if find_text_contains(&results, "空间站").is_none() {
        anyhow::bail!("当前不在空间站，无法开始游戏");
    }

    // OCR 识别屏幕
    let (rx, ry, rw, rh) = scale_region(1182, 0, 738, 1080);
    let results = ocr_screen(rx, ry, rw, rh, false, IS_DEBUG)?;

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
            click_at(scale_x(1362), scale_y(875));
            thread::sleep(Duration::from_millis(200));
            click_at(scale_x(1685), scale_y(930));
            thread::sleep(Duration::from_millis(200));
        }

        // 检测"开始"
        if result.text.contains("开始") {
            println!("[startGame] 找到 '开始'，点击");
            click_at(scale_x(1685), scale_y(930));
            thread::sleep(Duration::from_millis(200));
        }
    }

    let (rx, ry, rw, rh) = scale_region(674, 585, 570, 140);
    let results = ocr_screen(rx, ry, rw, rh, false, IS_DEBUG)?;
    for result in &results {
        if should_stop() {
            println!("[STOP] startGame: 检测到停止信号");
            return Ok(());
        }

        let (center_x, center_y) = result.center();

        if result.text.contains("今日不再提醒") {
            println!("[startGame] 找到 '今日不再提醒'，点击");
            click_at(scale_x(898), scale_y(609));
            thread::sleep(Duration::from_millis(200));
        }

        // 检测"开始"
        if result.text.contains("确认开启") {
            println!("[startGame] 找到 '确认开启'，点击");
            click_at(center_x, center_y);
            thread::sleep(Duration::from_millis(200));
        }
    }

    thread::sleep(Duration::from_secs(1));
    // 898,609
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

        let (fx, fy, fw, fh) = full_screen_region();
        let results = ocr_screen(fx, fy, fw, fh, false, IS_DEBUG)?;

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

/// 购买陷阱 - 默认顺序：防空导弹, 自修复磁暴塔, 破坏者, 修理站
pub fn buy_traps() -> Result<()> {
    buy_traps_ordered(&["防空导弹", "自修复磁暴塔", "破坏者", "修理站"])
}

/// 按指定顺序购买陷阱
///
/// 购买顺序决定热键分配：第一个→4键, 第二个→5键, 第三个→6键, 第四个→7键
///
/// 逻辑：打开商店（默认"全部"页面），按顺序逐个购买。
/// 如果当前页面找不到某个陷阱，依次切换到"地面"、"墙面"页面查找。
pub fn buy_traps_ordered(trap_names: &[&str]) -> Result<()> {
    if should_stop() {
        println!("[STOP] buy_traps: 检测到停止信号，跳过");
        return Ok(());
    }

    println!("[buy_traps] 打开商店，购买顺序: {:?}", trap_names);
    tap_key(VK_N);
    thread::sleep(Duration::from_secs(1));

    if should_stop() {
        tap_key(VK_N);
        return Ok(());
    }

    let (fx, fy, fw, fh) = full_screen_region();
    let tabs = ["地面", "墙面"];

    for trap_name in trap_names {
        if should_stop() {
            tap_key(VK_N);
            return Ok(());
        }

        let mut found = false;

        // 先在当前页面找
        let results = ocr_screen(fx, fy, fw, fh, false, IS_DEBUG)?;
        if let Some(result) = find_text_contains(&results, trap_name) {
            println!("[buy_traps] 在当前页面找到 '{}'，购买", trap_name);
            buy_trap_click(result.center());
            found = true;
        }

        // 当前页面没找到，依次切换"地面"、"墙面"
        if !found {
            for tab in &tabs {
                if should_stop() {
                    tap_key(VK_N);
                    return Ok(());
                }

                let results = ocr_screen(fx, fy, fw, fh, false, IS_DEBUG)?;
                if let Some(tab_result) = find_text_contains(&results, tab) {
                    println!("[buy_traps] 切换到 '{}' 页面", tab);
                    let (tx, ty) = tab_result.center();
                    click_at(tx, ty);
                    thread::sleep(Duration::from_millis(500));

                    let results = ocr_screen(fx, fy, fw, fh, false, IS_DEBUG)?;
                    if let Some(result) = find_text_contains(&results, trap_name) {
                        println!("[buy_traps] 在 '{}' 页面找到 '{}'，购买", tab, trap_name);
                        buy_trap_click(result.center());
                        found = true;
                        break;
                    }
                }
            }
        }

        if !found {
            println!("[buy_traps] 未找到 '{}', 跳过", trap_name);
        }
    }

    // 关闭商店
    tap_key(VK_N);
    Ok(())
}

/// 点击购买陷阱（内部辅助）
fn buy_trap_click((center_x, center_y): (i32, i32)) {
    move_to(center_x + scale_x(50), center_y + scale_y(50));
    thread::sleep(Duration::from_millis(300));
    left_click();
    thread::sleep(Duration::from_millis(300));
    left_click();
    thread::sleep(Duration::from_millis(300));
    left_click();
    thread::sleep(Duration::from_millis(500));
}

/// 批量放置陷阱（坐标为 1920x1080 基准，自动缩放到实际分辨率）
///
/// 内部循环调用 place_trap，每次放置前检查停止信号。
pub fn place_traps(positions: &[(i32, i32)], trap_key: &str) -> Result<()> {
    for (i, &(bx, by)) in positions.iter().enumerate() {
        if should_stop() {
            println!(
                "[STOP] place_traps: 第{}/{}个时停止",
                i + 1,
                positions.len()
            );
            return Ok(());
        }
        place_trap(scale_x(bx), scale_y(by), trap_key)?;
    }
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

        let (fx, fy, fw, fh) = full_screen_region();
        let results = ocr_screen(fx, fy, fw, fh, false, IS_DEBUG)?;

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
            move_to(x + scale_x(50), y + scale_y(50));
            thread::sleep(Duration::from_millis(200));
            left_click();
            thread::sleep(Duration::from_millis(200));
            left_click();
            thread::sleep(Duration::from_millis(200));
            left_click();
            thread::sleep(Duration::from_millis(500));
            continue;
        }

        // 检测"任务完成"并截图
        if find_text_contains(&results, "阶段完成").is_some() {
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
        // press_key(VK_D, 0.5);
        // press_key(VK_A, 0.5);
        press_key(VK_5, 5.0);
        press_key(VK_6, 5.0);
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

/// 等待金币达到指定数额
///
/// 循环检查后台监控的金币数，直到达到目标或收到停止信号。
pub fn wait_gold(amount: i64) -> Result<()> {
    println!("[wait_gold] 等待金币 >= {}", amount);
    loop {
        if should_stop() {
            println!("[STOP] wait_gold: 检测到停止信号");
            return Ok(());
        }

        let gold = monitor::current_gold();
        if gold >= amount {
            println!("[wait_gold] 金币 {} >= {}，继续", gold, amount);
            return Ok(());
        }

        thread::sleep(Duration::from_millis(500));
    }
}

/// 等待波次到达指定值
///
/// 循环检查后台监控的波次数，直到达到目标或收到停止信号。
pub fn wait_wave(wave: u32) -> Result<()> {
    println!("[wait_wave] 等待波次 >= {}", wave);
    loop {
        if should_stop() {
            println!("[STOP] wait_wave: 检测到停止信号");
            return Ok(());
        }

        let current = monitor::current_wave();
        if current >= wave {
            println!("[wait_wave] 波次 {} >= {}，继续", current, wave);
            return Ok(());
        }

        thread::sleep(Duration::from_millis(500));
    }
}

/// 放置陷阱（选择陷阱快捷键 + 点击坐标）
///
/// # Arguments
/// * `x`, `y` - 屏幕坐标
/// * `trap_key` - 陷阱快捷键字符串（如 "4", "5", "6", "7"）
pub fn place_trap(x: i32, y: i32, trap_key: &str) -> Result<()> {
    if should_stop() {
        return Ok(());
    }

    let vk = get_vk_code(trap_key).context(format!("未知的陷阱快捷键: {}", trap_key))?;
    println!("[place_trap] 放置陷阱 key={} @ ({}, {})", trap_key, x, y);

    tap_key(vk);
    thread::sleep(Duration::from_millis(1000));
    move_to(x, y);
    thread::sleep(Duration::from_millis(1000));
    left_click();
    thread::sleep(Duration::from_millis(200));
    left_click();
    thread::sleep(Duration::from_millis(300));
    Ok(())
}

/// 鼠标移动到某个坐标，放置某个陷阱
/// - x, y: 放置坐标
/// - trap_key: 陷阱快捷键 (如 VK_4, VK_5 等)
pub fn place_trap_at(x: i32, y: i32, trap_key: u16) -> Result<()> {
    tap_key(trap_key);
    thread::sleep(Duration::from_millis(1000));
    move_to(x, y);
    thread::sleep(Duration::from_millis(1000));
    left_click();
    thread::sleep(Duration::from_millis(200));
    left_click();
    thread::sleep(Duration::from_millis(300));
    Ok(())
}

/// 升级陷阱（长按热键 3 秒）
///
/// 陷阱的升级方式是长按对应热键，例如防空导弹是 4 键，长按 4 即升级。
/// trap_key: 陷阱快捷键字符串（如 "4", "5", "6", "7"）
pub fn upgrade_trap(trap_key: &str) -> Result<()> {
    if should_stop() {
        return Ok(());
    }
    let vk = get_vk_code(trap_key).context(format!("未知的陷阱快捷键: {}", trap_key))?;
    println!("[upgrade_trap] 长按 {} 升级", trap_key);
    press_key(vk, 3.0);
    thread::sleep(Duration::from_millis(500));
    Ok(())
}

/// 游戏动作枚举 - 用于批量执行并自动检查停止信号
pub enum GameAction {
    /// 按住键指定时间（秒）
    PressKey(u16, f64),
    /// 点击键
    TapKey(u16),
    /// 相对移动鼠标（视角转动）
    SendRelative(i32, i32),
    /// 等待指定时间（秒）
    Sleep(f64),
    /// 点击鼠标
    Click,
    /// 移动鼠标到坐标
    MoveTo(i32, i32),
}

/// 执行动作序列，每个动作后自动检查停止信号
///
/// # Returns
/// - Ok(true) 全部执行完成
/// - Ok(false) 检测到停止信号，提前退出
pub fn execute_actions(actions: &[GameAction]) -> Result<bool> {
    for action in actions {
        if should_stop() {
            println!("[STOP] execute_actions: 检测到停止信号");
            return Ok(false);
        }

        match action {
            GameAction::PressKey(vk, duration) => {
                press_key(*vk, *duration);
            }
            GameAction::TapKey(vk) => {
                tap_key(*vk);
            }
            GameAction::SendRelative(dx, dy) => {
                send_relative(*dx, *dy);
            }
            GameAction::Sleep(secs) => {
                thread::sleep(Duration::from_secs_f64(*secs));
            }
            GameAction::Click => {
                left_click();
            }
            GameAction::MoveTo(x, y) => {
                move_to(*x, *y);
            }
        }
    }
    Ok(true)
}
