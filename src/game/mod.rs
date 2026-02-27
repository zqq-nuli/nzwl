//! 游戏自动化逻辑模块
//!
//! 每个地图/难度是一个独立模块，包含按波次组织的函数。
//! 通过 `available_maps()` 注册所有可用地图，供 GUI 下拉框使用。

pub mod building_inferno;
pub mod common;
pub mod training_hard;

use anyhow::Result;

/// 地图信息（供 GUI 下拉框选择）
pub struct MapInfo {
    /// 显示名称
    pub name: &'static str,
    /// 难度
    pub difficulty: &'static str,
    /// 开始游戏函数
    pub start_fn: fn() -> Result<()>,
    /// 执行所有波次函数
    pub waves_fn: fn() -> Result<()>,
}

/// 获取所有可用地图
pub fn available_maps() -> Vec<MapInfo> {
    vec![
        MapInfo {
            name: "训练基地",
            difficulty: "困难",
            start_fn: training_hard::start_game,
            waves_fn: training_hard::run_all_waves,
        },
        MapInfo {
            name: "大厦",
            difficulty: "炼狱",
            start_fn: building_inferno::start_game,
            waves_fn: building_inferno::run_all_waves,
        },
    ]
}
