//! 策略数据模型
//!
//! 定义地图策略的 JSON 结构，编辑器和执行器共用。

use serde::{Deserialize, Serialize};
use std::path::Path;

/// 根策略结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Strategy {
    pub meta: StrategyMeta,
    /// 商店购买顺序（陷阱名称列表）
    pub shop_order: Vec<String>,
    /// 建筑放置列表
    pub buildings: Vec<Building>,
    /// 升级事件
    #[serde(default)]
    pub upgrades: Vec<UpgradeEvent>,
    /// 拆除事件
    #[serde(default)]
    pub demolishes: Vec<DemolishEvent>,
    /// 移动阶段
    #[serde(default)]
    pub movement_phases: Vec<MovementPhase>,
}

/// 策略元信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StrategyMeta {
    pub name: String,
    pub difficulty: String,
    /// 截图路径（编辑器用）
    #[serde(default)]
    pub screenshot: String,
    /// 网格像素大小（编辑器用）
    #[serde(default = "default_grid_size")]
    pub grid_pixel_size: f32,
    /// 网格 X 偏移（编辑器用）
    #[serde(default)]
    pub offset_x: f32,
    /// 网格 Y 偏移（编辑器用）
    #[serde(default)]
    pub offset_y: f32,
}

fn default_grid_size() -> f32 {
    64.0
}

/// 建筑（陷阱）放置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Building {
    /// 唯一标识
    pub id: String,
    /// 显示名称
    pub name: String,
    /// 陷阱快捷键（如 "5"、"6"）
    pub trap_key: String,
    /// 屏幕 X 坐标（执行用）
    pub screen_x: i32,
    /// 屏幕 Y 坐标（执行用）
    pub screen_y: i32,
    /// 网格 X 坐标（编辑器显示用）
    #[serde(default)]
    pub grid_x: f32,
    /// 网格 Y 坐标（编辑器显示用）
    #[serde(default)]
    pub grid_y: f32,
    /// 放置波次（1 = 波次1前放置）
    pub wave: u32,
    /// 是否在波次开始后放置（需要 OCR 等待波次号）
    #[serde(default)]
    pub is_late: bool,
}

impl Building {
    /// 排序键：wave * 2 + is_late，用于确定执行顺序
    pub fn sort_key(&self) -> u32 {
        self.wave * 2 + self.is_late as u32
    }
}

/// 升级事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpgradeEvent {
    pub building_id: String,
    pub wave: u32,
    #[serde(default)]
    pub is_late: bool,
}

/// 拆除事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DemolishEvent {
    pub building_id: String,
    pub wave: u32,
    #[serde(default)]
    pub is_late: bool,
}

/// 移动阶段
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MovementPhase {
    /// 阶段名称
    pub name: String,
    /// 触发时机，如 "before_wave_1"、"after_placement"
    pub trigger: String,
    /// 动作序列
    pub actions: Vec<ActionStep>,
}

/// 动作步骤（带 serde tag）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ActionStep {
    /// 按住键指定秒数
    PressKey { key: String, duration: f64 },
    /// 点击键
    TapKey { key: String },
    /// 按下键（不松开）
    KeyDown { key: String },
    /// 松开键
    KeyUp { key: String },
    /// 相对移动鼠标（视角转动）
    SendRelative { dx: i32, dy: i32 },
    /// 等待（秒）
    Sleep { duration: f64 },
    /// 鼠标左键点击
    Click,
    /// 移动鼠标到绝对坐标
    MoveTo { x: i32, y: i32 },
    /// 移动并点击
    ClickAt { x: i32, y: i32 },
}

// ===== 坐标转换 =====

/// 网格坐标 → 屏幕像素坐标
pub fn grid_to_screen(grid_x: f32, grid_y: f32, meta: &StrategyMeta) -> (i32, i32) {
    let sx = (grid_x * meta.grid_pixel_size + meta.offset_x) as i32;
    let sy = (grid_y * meta.grid_pixel_size + meta.offset_y) as i32;
    (sx, sy)
}

/// 屏幕像素坐标 → 网格坐标
pub fn screen_to_grid(screen_x: i32, screen_y: i32, meta: &StrategyMeta) -> (f32, f32) {
    let gx = (screen_x as f32 - meta.offset_x) / meta.grid_pixel_size;
    let gy = (screen_y as f32 - meta.offset_y) / meta.grid_pixel_size;
    (gx, gy)
}

// ===== JSON 读写 =====

impl Strategy {
    /// 从 JSON 文件加载策略
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let strategy: Strategy = serde_json::from_str(&content)?;
        Ok(strategy)
    }

    /// 保存策略到 JSON 文件
    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }
}
