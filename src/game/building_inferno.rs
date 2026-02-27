//! 大厦 - 炼狱难度
//!
//! 每个波次一个函数，可单独调试。
//! 调试时在 run_all_waves() 中注释掉不需要的波次即可。
//!
//! 装备顺序（购买顺序决定热键）:
//!   天网 → 4键, 自修复磁暴塔 → 5键, 天启 → 6键, 防空导弹 → 7键

use anyhow::{Context, Result};
use std::thread;
use std::time::Duration;

use super::common::{
    buy_traps_ordered, find_game_window, place_trap, setup_window, upgrade_trap, wait_for_game_end,
    wait_gold, wait_wave, IS_DEBUG,
};
use crate::input::{click_at, press_key, VK_SPACE};
use crate::ocr::{find_text_contains, ocr_screen};
use crate::screen::dev_x;
use crate::screen::dev_y;
use crate::screen::full_screen_region;
use crate::stop_flag::should_stop;

// ===== 陷阱热键 =====

const TIANWANG: &str = "4"; // 天网
const CIBAO: &str = "5"; // 自修复磁暴塔
const TIANQI: &str = "6"; // 天启
const FANGKONG: &str = "7"; // 防空导弹

const EQUIPPED_TRAPS: &[&str] = &["天网", "自修复磁暴塔", "天启", "防空导弹"];

// ===== 开始游戏 =====

pub fn start_game() -> Result<()> {
    println!("[大厦:炼狱] 开始游戏...");

    // 查找并设置游戏窗口
    let hwnd = find_game_window().context("未找到游戏窗口 '逆战：未来'")?;
    setup_window(hwnd)?;

    // 1. 全屏 OCR，确认在正确界面
    let (fx, fy, fw, fh) = full_screen_region();
    let mut results = ocr_screen(fx, fy, fw, fh, false, IS_DEBUG)?;

    // 如果出现"挑战模式"，点击切换到经典模式
    if find_text_contains(&results, "挑战模式").is_some() {
        println!("[大厦:炼狱] 检测到 '挑战模式'，切换到经典模式");
        click_at(dev_x(2906), dev_y(443));
        thread::sleep(Duration::from_millis(500));
        results = ocr_screen(fx, fy, fw, fh, false, IS_DEBUG)?;
    }

    if find_text_contains(&results, "联盟大厦").is_none()
        || find_text_contains(&results, "经典模式").is_none()
    {
        anyhow::bail!("未找到 '联盟大厦' 或 '经典模式'，请确认在正确界面");
    }
    println!("[大厦:炼狱] 确认界面: 联盟大厦 - 经典模式");

    // 2. 点击"炼狱"
    if let Some(r) = find_text_contains(&results, "炼狱") {
        let (cx, cy) = r.center();
        println!("[大厦:炼狱] 点击 '炼狱' @ ({},{})", cx, cy);
        click_at(cx, cy);
        thread::sleep(Duration::from_millis(500));
    } else {
        anyhow::bail!("未找到 '炼狱' 难度选项");
    }

    // 3. 判断是否有"创建房间"，有则点击"单人挑战"
    let results = ocr_screen(fx, fy, fw, fh, false, IS_DEBUG)?;
    if find_text_contains(&results, "创建房间").is_some() {
        if let Some(r) = find_text_contains(&results, "单人挑战") {
            let cx = dev_x(2665);
            let cy = dev_y(1772);
            println!("[大厦:炼狱] 点击 '单人挑战' @ ({},{})", cx, cy);
            click_at(cx, cy);
            thread::sleep(Duration::from_millis(500));
        }
    }

    // 4. 再次判断，没有"创建房间"则点击"开始"
    let results = ocr_screen(fx, fy, fw, fh, false, IS_DEBUG)?;
    if find_text_contains(&results, "创建房间").is_none() {
        if let Some(r) = find_text_contains(&results, "开始") {
            let (cx, cy) = r.center();
            println!("[大厦:炼狱] 点击 '开始' @ ({},{})", cx, cy);
            click_at(cx, cy);
            thread::sleep(Duration::from_millis(500));
        }
    }

    // 5. 等待出现"长按跳过"，然后长按空格跳过
    println!("[大厦:炼狱] 等待 '长按跳过'...");
    loop {
        if should_stop() {
            println!("[STOP] start_game: 检测到停止信号");
            return Ok(());
        }
        let results = ocr_screen(fx, fy, fw, fh, false, IS_DEBUG)?;
        if find_text_contains(&results, "跳过").is_some() {
            println!("[大厦:炼狱] 找到 '跳过'，长按空格");
            press_key(VK_SPACE, 3.0);
            break;
        }
        thread::sleep(Duration::from_secs(1));
    }

    // 6. 等待波次为 1
    println!("[大厦:炼狱] 等待波次 1...");
    wait_wave(1)?;
    println!("[大厦:炼狱] 游戏开始！");

    Ok(())
}

// ===== 波次 1 =====

pub fn wave_1() -> Result<()> {
    if should_stop() {
        return Ok(());
    }
    println!("[大厦:炼狱] === 波次 1 ===");

    buy_traps_ordered(EQUIPPED_TRAPS)?;

    Ok(())
}

// ===== 波次 2 =====

pub fn wave_2() -> Result<()> {
    if should_stop() {
        return Ok(());
    }
    println!("[大厦:炼狱] === 波次 2 ===");

    Ok(())
}

// ===== 波次 3 =====

pub fn wave_3() -> Result<()> {
    if should_stop() {
        return Ok(());
    }
    println!("[大厦:炼狱] === 波次 3 ===");

    Ok(())
}

// ===== 波次 4 =====

pub fn wave_4() -> Result<()> {
    if should_stop() {
        return Ok(());
    }
    println!("[大厦:炼狱] === 波次 4 ===");

    Ok(())
}

// ===== 波次 5 =====

pub fn wave_5() -> Result<()> {
    if should_stop() {
        return Ok(());
    }
    println!("[大厦:炼狱] === 波次 5 ===");

    Ok(())
}

// ===== 波次 6 =====

pub fn wave_6() -> Result<()> {
    if should_stop() {
        return Ok(());
    }
    println!("[大厦:炼狱] === 波次 6 ===");

    Ok(())
}

// ===== 波次 7 =====

pub fn wave_7() -> Result<()> {
    if should_stop() {
        return Ok(());
    }
    println!("[大厦:炼狱] === 波次 7 ===");

    Ok(())
}

// ===== 波次 8 =====

pub fn wave_8() -> Result<()> {
    if should_stop() {
        return Ok(());
    }
    println!("[大厦:炼狱] === 波次 8 ===");

    Ok(())
}

// ===== 波次 9 (Boss) =====

pub fn wave_9() -> Result<()> {
    if should_stop() {
        return Ok(());
    }
    println!("[大厦:炼狱] === 波次 9 (Boss) ===");

    Ok(())
}

// ===== 执行所有波次 =====

/// 调试时注释掉不需要的波次，只跑某一段。
pub fn run_all_waves() -> Result<()> {
    wave_1()?;

    wait_wave(2)?;
    wave_2()?;

    wait_wave(3)?;
    wave_3()?;

    wait_wave(4)?;
    wave_4()?;

    wait_wave(5)?;
    wave_5()?;

    wait_wave(6)?;
    wave_6()?;

    wait_wave(7)?;
    wave_7()?;

    wait_wave(8)?;
    wave_8()?;

    wait_wave(9)?;
    wave_9()?;

    wait_for_game_end()?;
    Ok(())
}
