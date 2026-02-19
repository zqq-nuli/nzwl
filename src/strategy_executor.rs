//! 策略执行器
//!
//! 读取 JSON 策略文件并执行，替代硬编码的 main_game_loop()。

use anyhow::Result;
use std::thread;
use std::time::Duration;

use crate::input::{
    click_at, get_vk_code, key_down, key_up, left_click, move_to, press_key, send_relative,
    tap_key, VK_G, VK_N, VK_O,
};
use crate::ocr::{find_text_contains, ocr_screen};
use crate::stop_flag::should_stop;
use crate::strategy::{ActionStep, Strategy};

use crate::game::common::{
    clear_cache, start_game_with_difficulty, wait_for_game_end, IS_DEBUG,
};

/// 解析快捷键字符串为虚拟键码
fn resolve_key(key: &str) -> Result<u16> {
    get_vk_code(key).ok_or_else(|| anyhow::anyhow!("未知按键: {}", key))
}

/// 执行单个动作步骤
fn execute_step(step: &ActionStep) -> Result<()> {
    match step {
        ActionStep::PressKey { key, duration } => {
            let vk = resolve_key(key)?;
            press_key(vk, *duration);
        }
        ActionStep::TapKey { key } => {
            let vk = resolve_key(key)?;
            tap_key(vk);
        }
        ActionStep::KeyDown { key } => {
            let vk = resolve_key(key)?;
            key_down(vk);
        }
        ActionStep::KeyUp { key } => {
            let vk = resolve_key(key)?;
            key_up(vk);
        }
        ActionStep::SendRelative { dx, dy } => {
            send_relative(*dx, *dy);
        }
        ActionStep::Sleep { duration } => {
            thread::sleep(Duration::from_secs_f64(*duration));
        }
        ActionStep::Click => {
            left_click();
        }
        ActionStep::MoveTo { x, y } => {
            move_to(*x, *y);
        }
        ActionStep::ClickAt { x, y } => {
            click_at(*x, *y);
        }
    }
    Ok(())
}

/// 执行动作序列，每步之间检查停止信号
fn execute_actions(actions: &[ActionStep]) -> Result<bool> {
    for step in actions {
        if should_stop() {
            return Ok(false);
        }
        execute_step(step)?;
    }
    Ok(true)
}

/// 执行指定 trigger 的移动阶段
fn run_movement_phase(strategy: &Strategy, trigger: &str) -> Result<bool> {
    for phase in &strategy.movement_phases {
        if phase.trigger == trigger {
            println!("[executor] 执行移动阶段: {}", phase.name);
            if !execute_actions(&phase.actions)? {
                return Ok(false);
            }
        }
    }
    Ok(true)
}

/// 基于 OCR 按指定顺序购买陷阱
fn buy_traps_from_list(shop_order: &[String]) -> Result<()> {
    if should_stop() {
        return Ok(());
    }

    println!("[executor] 打开商店");
    tap_key(VK_N);
    thread::sleep(Duration::from_secs(1));

    if should_stop() {
        tap_key(VK_N);
        return Ok(());
    }

    let results = ocr_screen(0, 0, 1920, 1080, false, IS_DEBUG)?;
    thread::sleep(Duration::from_millis(1000));

    for trap_name in shop_order {
        if should_stop() {
            tap_key(VK_N);
            return Ok(());
        }

        if let Some(result) = find_text_contains(&results, trap_name) {
            println!("[executor] 购买: {}", trap_name);
            let (cx, cy) = result.center();
            move_to(cx + 50, cy + 50);
            thread::sleep(Duration::from_millis(300));
            left_click();
            thread::sleep(Duration::from_millis(300));
            left_click();
            thread::sleep(Duration::from_millis(300));
            left_click();
            thread::sleep(Duration::from_millis(500));
        } else {
            println!("[executor] 未找到: {}", trap_name);
        }
    }

    tap_key(VK_N);
    Ok(())
}

/// 放置单个建筑
fn place_building(building: &crate::strategy::Building) -> Result<()> {
    let vk = resolve_key(&building.trap_key)?;
    tap_key(vk);
    thread::sleep(Duration::from_millis(300));
    move_to(building.screen_x, building.screen_y);
    thread::sleep(Duration::from_millis(200));
    left_click();
    thread::sleep(Duration::from_millis(200));
    left_click();
    thread::sleep(Duration::from_millis(300));
    Ok(())
}

/// 等待指定波次出现（OCR 检测）
fn wait_for_wave(wave: u32) -> Result<bool> {
    let target = format!("波次{}", wave);
    println!("[executor] 等待 {} ...", target);

    loop {
        if should_stop() {
            return Ok(false);
        }

        let results = ocr_screen(0, 0, 420, 320, false, IS_DEBUG)?;

        // 处理"返回游戏"弹窗
        if let Some(result) = find_text_contains(&results, "返回游戏") {
            let (x, y) = result.center();
            move_to(x, y);
            thread::sleep(Duration::from_millis(200));
            left_click();
            thread::sleep(Duration::from_millis(500));
        }

        if find_text_contains(&results, &target).is_some() {
            println!("[executor] 检测到 {}", target);
            return Ok(true);
        }

        thread::sleep(Duration::from_secs(2));
    }
}

/// 主策略执行函数
pub fn run_strategy(strategy: &Strategy) -> Result<()> {
    if should_stop() {
        return Ok(());
    }

    println!("[executor] 开始执行策略: {}", strategy.meta.name);
    clear_cache();

    // 1. 购买陷阱
    buy_traps_from_list(&strategy.shop_order)?;
    if should_stop() {
        return Ok(());
    }

    // 2. 进入放置模式
    println!("[executor] 进入放置模式");
    tap_key(VK_O);
    thread::sleep(Duration::from_millis(500));

    // 3. 按 sort_key 排序建筑
    let mut sorted_buildings = strategy.buildings.clone();
    sorted_buildings.sort_by_key(|b| b.sort_key());

    // 按波次分组执行
    let mut current_wave: u32 = 0;
    let mut wave_started = false;

    for building in &sorted_buildings {
        if should_stop() {
            tap_key(VK_O);
            return Ok(());
        }

        // 新波次开始
        if building.wave != current_wave {
            current_wave = building.wave;
            wave_started = false;

            // 执行 before_wave_N 移动阶段
            let trigger = format!("before_wave_{}", current_wave);
            if !run_movement_phase(strategy, &trigger)? {
                tap_key(VK_O);
                return Ok(());
            }
        }

        // 如果是 is_late 建筑，需要等待波次出现
        if building.is_late && !wave_started {
            // 先开始波次（如果是第一波）
            if current_wave == 1 {
                tap_key(VK_G);
            }

            // 执行 wait_wave_N 移动阶段（巡逻等待）
            let wait_trigger = format!("wait_wave_{}", current_wave);
            // 在等待波次的同时执行巡逻动作
            let target = format!("波次{}", current_wave);
            println!("[executor] 等待 {} ...", target);

            loop {
                if should_stop() {
                    tap_key(VK_O);
                    return Ok(());
                }

                // 执行巡逻动作
                run_movement_phase(strategy, &wait_trigger)?;

                let results = ocr_screen(0, 0, 420, 320, false, IS_DEBUG)?;

                if let Some(result) = find_text_contains(&results, "返回游戏") {
                    let (x, y) = result.center();
                    move_to(x, y);
                    thread::sleep(Duration::from_millis(200));
                    left_click();
                    thread::sleep(Duration::from_millis(500));
                }

                if find_text_contains(&results, &target).is_some() {
                    println!("[executor] 检测到 {}", target);
                    break;
                }
            }

            wave_started = true;

            // 执行 during_wave_N 移动阶段
            let during_trigger = format!("during_wave_{}", current_wave);
            if !run_movement_phase(strategy, &during_trigger)? {
                tap_key(VK_O);
                return Ok(());
            }
        }

        // 放置建筑
        println!(
            "[executor] 放置 {} @ ({}, {}) wave={}",
            building.name, building.screen_x, building.screen_y, building.wave
        );
        place_building(building)?;
    }

    // 如果有波次1的非 late 建筑但还没开始，开始第一波
    if sorted_buildings.iter().any(|b| b.wave == 1 && !b.is_late)
        && !sorted_buildings.iter().any(|b| b.is_late)
    {
        tap_key(VK_G);
    }

    // 4. 退出放置模式
    println!("[executor] 退出放置模式");
    tap_key(VK_O);
    thread::sleep(Duration::from_millis(500));

    if should_stop() {
        return Ok(());
    }

    // 5. 执行 after_placement 移动阶段（去安全点）
    if !run_movement_phase(strategy, "after_placement")? {
        return Ok(());
    }

    if should_stop() {
        return Ok(());
    }

    // 6. 等待游戏结束
    wait_for_game_end()?;

    println!("[executor] 策略执行完成: {}", strategy.meta.name);
    Ok(())
}

/// 使用策略执行完整的一轮游戏（start_game + run_strategy）
pub fn start_game_with_strategy(strategy: &Strategy) -> Result<()> {
    start_game_with_difficulty(&strategy.meta.difficulty)?;
    if should_stop() {
        return Ok(());
    }
    run_strategy(strategy)
}
