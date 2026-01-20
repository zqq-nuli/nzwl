//! 炼狱版游戏逻辑
//!
//! 炼狱难度的自动化流程
//! TODO: 根据炼狱版实际情况修改陷阱位置和安全点

#![allow(dead_code)]

use anyhow::Result;
use std::thread;
use std::time::Duration;

use crate::keys::{
    left_click_legacy, move_down, move_left, move_to, press_key, press_key_sequence, tap_key,
    KeyAction, VK_4, VK_5, VK_A, VK_D, VK_G, VK_O, VK_S, VK_SPACE, VK_W,
};
use crate::ocr::{find_text_contains, ocr_screen};
use crate::stop_flag::should_stop;

use super::common::{
    buy_traps, clear_cache, start_game_with_difficulty, wait_for_game_end, IS_DEBUG, MOVE_VALUE,
};

/// 炼狱版难度标识
const DIFFICULTY: &str = "炼狱";

/// 开始游戏 - 炼狱版
pub fn start_game() -> Result<()> {
    start_game_with_difficulty(DIFFICULTY)
}

/// 放置首关陷阱 - 炼狱版
/// TODO: 根据炼狱版地图修改陷阱位置
fn place_first_level_traps() -> Result<()> {
    if should_stop() {
        println!("[STOP] place_first_level_traps: 检测到停止信号，跳过");
        return Ok(());
    }

    println!("[place_traps] 进入放置模式 - 炼狱版");
    tap_key(VK_O);
    thread::sleep(Duration::from_millis(500));

    if should_stop() {
        tap_key(VK_O);
        return Ok(());
    }

    // TODO: 修改为炼狱版的移动和陷阱位置
    // 移动到位置
    press_key(VK_S, 5.0);
    press_key(VK_A, 5.0);

    // TODO: 修改为炼狱版的陷阱坐标
    let left_points: [(i32, i32); 2] = [(1055, 525), (1221, 525)];
    tap_key(VK_4);
    thread::sleep(Duration::from_millis(300));

    for (x, y) in left_points {
        if should_stop() {
            tap_key(VK_O);
            return Ok(());
        }

        move_to(x, y);
        thread::sleep(Duration::from_millis(200));
        left_click_legacy();
        thread::sleep(Duration::from_millis(200));
        left_click_legacy();
        thread::sleep(Duration::from_millis(300));
    }

    if should_stop() {
        tap_key(VK_O);
        return Ok(());
    }

    // 移动到右侧
    press_key(VK_D, 5.0);

    // TODO: 修改为炼狱版的右侧陷阱坐标
    let right_points: [(i32, i32); 2] = [(857, 532), (687, 532)];

    for (x, y) in right_points {
        if should_stop() {
            tap_key(VK_O);
            return Ok(());
        }

        move_to(x, y);
        thread::sleep(Duration::from_millis(200));
        left_click_legacy();
        thread::sleep(Duration::from_millis(200));
        left_click_legacy();
        thread::sleep(Duration::from_millis(300));
    }

    if should_stop() {
        tap_key(VK_O);
        return Ok(());
    }

    // 开始第一波次
    tap_key(VK_G);

    // TODO: 修改为炼狱版的炮台位置
    let left_paotai: [(i32, i32); 2] = [(1396, 359), (1393, 188)];
    let right_paotai: [(i32, i32); 2] = [(518, 365), (516, 194)];

    tap_key(VK_5);
    println!("[place_traps] 等待波次2出现...");

    // 等待波次2
    loop {
        if should_stop() {
            println!("[STOP] place_traps: 检测到停止信号");
            break;
        }

        press_key(VK_A, 0.5);
        thread::sleep(Duration::from_millis(500));
        press_key(VK_D, 2.0);
        thread::sleep(Duration::from_secs(2));

        let results = ocr_screen(0, 0, 420, 320, false, IS_DEBUG)?;

        println!("[OCR] 检测到 {} 个文字块:", results.len());
        for r in &results {
            println!("  - '{}'", r.text);
        }

        // 处理"返回游戏"弹窗
        if let Some(result) = find_text_contains(&results, "返回游戏") {
            let (x, y) = result.center();
            move_to(x, y);
            thread::sleep(Duration::from_millis(200));
            left_click_legacy();
            thread::sleep(Duration::from_millis(200));
            left_click_legacy();
            thread::sleep(Duration::from_millis(200));
            left_click_legacy();
            thread::sleep(Duration::from_millis(500));
        }

        // 检测波次2
        if find_text_contains(&results, "波次2").is_some() {
            println!("[place_traps] 检测到波次2，放置炮台");

            // 放置右侧炮台
            for (x, y) in right_paotai {
                move_to(x, y);
                thread::sleep(Duration::from_millis(200));
                left_click_legacy();
                thread::sleep(Duration::from_millis(200));
                left_click_legacy();
                thread::sleep(Duration::from_millis(300));
            }

            // 移动到左侧
            press_key(VK_A, 5.0);

            // 放置左侧炮台
            for (x, y) in left_paotai {
                move_to(x, y);
                thread::sleep(Duration::from_millis(200));
                left_click_legacy();
                thread::sleep(Duration::from_millis(200));
                left_click_legacy();
                thread::sleep(Duration::from_millis(300));
            }

            break;
        }
    }

    println!("[place_traps] 首关陷阱放置完成 - 炼狱版");
    tap_key(VK_O);
    thread::sleep(Duration::from_millis(500));

    Ok(())
}

/// 去安全点 - 炼狱版
/// TODO: 根据炼狱版地图修改安全点位置
fn goto_safe_point() -> Result<()> {
    if should_stop() {
        println!("[STOP] goto_safe_point: 检测到停止信号，跳过");
        return Ok(());
    }

    println!("[goto_safe_point] 移动到安全点 - 炼狱版");

    // TODO: 修改为炼狱版的安全点移动路线
    move_left(MOVE_VALUE * 50);
    press_key(VK_W, 2.0);
    move_left(MOVE_VALUE * 50);
    press_key(VK_W, 5.0);

    if should_stop() {
        return Ok(());
    }

    press_key_sequence(&[
        KeyAction::Hold(VK_W, 0.0),
        KeyAction::Tap(VK_SPACE, 2),
        KeyAction::Release(VK_W),
    ]);

    press_key(VK_W, 1.0);
    press_key(VK_D, 1.0);
    thread::sleep(Duration::from_secs(1));

    if should_stop() {
        return Ok(());
    }

    move_left(MOVE_VALUE * 76);
    move_down(MOVE_VALUE * 2);

    Ok(())
}

/// 主游戏流程 - 炼狱版
pub fn main_game_loop() -> Result<()> {
    if should_stop() {
        println!("[STOP] main: 检测到停止信号，跳过执行");
        return Ok(());
    }

    println!("[main] 开始游戏流程 - 炼狱版");

    // 清空 OCR 缓存
    clear_cache();

    // 1. 购买陷阱
    buy_traps()?;

    if should_stop() {
        return Ok(());
    }

    // 2. 放置陷阱
    place_first_level_traps()?;

    if should_stop() {
        return Ok(());
    }

    // 3. 移动到安全点
    goto_safe_point()?;

    if should_stop() {
        return Ok(());
    }

    // 4. 等待游戏结束
    wait_for_game_end()?;

    println!("[main] 游戏流程完成 - 炼狱版");
    Ok(())
}
