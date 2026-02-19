//! 炼狱版游戏逻辑
//!
//! 炼狱难度的自动化流程
//! TODO: 根据炼狱版实际情况修改陷阱位置和安全点

#![allow(dead_code)]

use anyhow::Result;
use std::thread;
use std::time::Duration;

use crate::input::{
    key_down, key_up, left_click, mouse_scroll, move_down, move_left, move_to, press_key,
    press_key_sequence, tap_key, KeyAction, ScrollDirection, VK_3, VK_5, VK_6,
    VK_7, VK_A, VK_D, VK_E, VK_G, VK_O, VK_S, VK_SPACE, VK_W,
};
use crate::screen::{check_pixel_color, get_pixel_color};
use crate::stop_flag::should_stop;

use super::common::{
    buy_traps, clear_cache, execute_actions, place_trap_at, start_game_with_difficulty,
    wait_for_game_end, GameAction, IS_DEBUG, MOVE_VALUE,
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
    loop {
        if should_stop() {
            println!("[STOP] place_first_level_traps: 检测到停止信号，跳出放置循环");
            break;
        }

        tap_key(VK_O); // 单击O打开平面地图
        thread::sleep(Duration::from_millis(500));

        let color = get_pixel_color(917, 870)?;
        // 鼠标向下滚动20次
        mouse_scroll(ScrollDirection::Down, 200, 0.01);
        thread::sleep(Duration::from_millis(500));
        move_to(1920 / 2, 1080 / 2);
        left_click();
        if check_pixel_color(917, 870, color)? {
            println!("颜色匹配");
            // 按下S 3秒
            press_key(VK_S, 3.0);
            break;
        }
        tap_key(VK_O); // 单击O打开平面地图
    }
    if should_stop() {
        tap_key(VK_O);
        return Ok(());
    }

    // 单击 5 键 放置 自修复
    place_trap_at(987,232, VK_5)?; // 放置自修复
    place_trap_at(923,316, VK_6)?; // 放置破坏者
    place_trap_at(987,293, VK_7)?; // 放置维修
    place_trap_at(1050,316, VK_6)?; //放置破坏者
    tap_key(VK_G); // 开始第一波

    // 退出平面地图并移动到安全位置
    execute_actions(&[
        GameAction::TapKey(VK_O),
        GameAction::Sleep(0.2),
        GameAction::TapKey(VK_3),
        GameAction::Sleep(0.5),
        GameAction::PressKey(VK_A, 1.5),
        GameAction::Sleep(0.5),
        GameAction::PressKey(VK_S, 4.0),
        GameAction::Sleep(0.5),
        GameAction::PressKey(VK_D, 2.0),
        GameAction::Sleep(0.5),
        GameAction::PressKey(VK_S, 4.0),
        GameAction::Sleep(0.5),
        GameAction::SendRelative(2237, 0), // 转 180 度
        GameAction::Sleep(0.5),
        GameAction::PressKey(VK_W, 1.0),
        GameAction::Sleep(0.5),
        GameAction::TapKey(VK_E),
        GameAction::Sleep(2.0),
        GameAction::SendRelative(-284, 0), // 转 -35 度
        GameAction::Sleep(0.5),
    ])?;

    if should_stop() {
        return Ok(());
    }

    // 跳跃前进
    key_down(VK_W);
    tap_key(VK_SPACE);
    thread::sleep(Duration::from_millis(1000));
    key_up(VK_W);
    thread::sleep(Duration::from_millis(300));
    key_down(VK_W);
    tap_key(VK_SPACE);
    thread::sleep(Duration::from_millis(1000));
    key_up(VK_W);
    thread::sleep(Duration::from_millis(300));
    key_down(VK_W);
    tap_key(VK_SPACE);
    thread::sleep(Duration::from_millis(1000));
    key_up(VK_W);

    if should_stop() {
        return Ok(());
    }

    // 移动到最终的安全区域
    execute_actions(&[
        GameAction::PressKey(VK_W, 1.5),
        GameAction::PressKey(VK_W, 2.0),
        GameAction::Sleep(0.5),
        GameAction::PressKey(VK_D, 2.0),
        GameAction::Sleep(0.5),
        GameAction::PressKey(VK_S, 4.0),
        GameAction::Sleep(0.5),
    ])?;

    if should_stop() {
        return Ok(());
    }

    // 斜向移动
    key_down(VK_S);
    thread::sleep(Duration::from_millis(500));
    key_down(VK_D);
    press_key(VK_S, 8.0);
    key_up(VK_S);
    key_up(VK_D);

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

    // 放置陷阱-波次1
    place_first_level_traps()?;

    if should_stop() {
        return Ok(());
    }

    // // 3. 移动到安全点
    // goto_safe_point()?;

    // if should_stop() {
    //     return Ok(());
    // }

    // // 4. 等待游戏结束
    // wait_for_game_end()?;

    println!("[main] 游戏流程完成 - 炼狱版");
    Ok(())
}
