//! 训练基地 - 困难难度
//!
//! 每个波次一个函数，可单独调试。
//! 调试时注释掉不需要的波次即可。

use anyhow::Result;

use super::common::{
    buy_traps, place_trap, start_game_with_difficulty, wait_for_game_end, wait_gold, wait_wave,
};
use crate::stop_flag::should_stop;

/// 开始游戏
pub fn start_game() -> Result<()> {
    start_game_with_difficulty("困难")
}

/// 波次 1：初始陷阱布置
pub fn wave_1() -> Result<()> {
    if should_stop() {
        return Ok(());
    }
    println!("[训练基地:困难] === 波次 1 ===");

    buy_traps()?;

    // TODO: 根据实际游戏调整坐标和金币阈值
    // wait_gold(2500)?;
    // place_trap(800, 400, "5")?;

    Ok(())
}

/// 波次 2
pub fn wave_2() -> Result<()> {
    if should_stop() {
        return Ok(());
    }
    println!("[训练基地:困难] === 波次 2 ===");

    // TODO: 根据实际游戏调整
    // wait_gold(5000)?;
    // place_trap(600, 300, "6")?;

    Ok(())
}

/// 波次 3（Boss 示例）
pub fn wave_3_boss() -> Result<()> {
    if should_stop() {
        return Ok(());
    }
    println!("[训练基地:困难] === 波次 3 (Boss) ===");

    // Boss 波次可以写任意复杂逻辑
    // wait_gold(15000)?;
    // place_trap(400, 300, "7")?;
    // for _ in 0..3 {
    //     move_to(960, 540);
    //     left_click();
    //     thread::sleep(Duration::from_secs(1));
    // }

    Ok(())
}

/// 执行所有波次
///
/// 调试时可以注释掉前面的波次，只跑某一段。
pub fn run_all_waves() -> Result<()> {
    wave_1()?;

    wait_wave(2)?;
    wave_2()?;

    wait_wave(3)?;
    wave_3_boss()?;

    wait_for_game_end()?;
    Ok(())
}
