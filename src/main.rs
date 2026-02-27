#![windows_subsystem = "windows"]
//! nz-rust: 逆战：未来 游戏自动化工具 (Rust 版)
//!
//! GUI 主程序，包含：
//! - 地图/难度选择
//! - 启动/停止控制
//! - 实时波次/金币显示
//! - 日志面板
//! - OCR 区域配置（持久化到 settings.ini）

mod game;
mod input;
mod keys;
mod logitech;
mod monitor;
mod ocr;
mod screen;
mod stop_flag;

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use eframe::egui;

use crate::game::available_maps;
use crate::game::common::buy_traps;
use crate::input::click_at;
use crate::monitor::MonitorConfig;
use crate::ocr::{ocr_screen, OcrResultItem};
use crate::screen::{get_scale_factors, get_screen_resolution};
use crate::stop_flag::{request_stop, reset_stop, should_stop};

/// 热键事件信号：0=无, 1=F1(启动), 2=F2(停止)
static HOTKEY_EVENT: AtomicU8 = AtomicU8::new(0);

/// 游戏是否正在运行
static GAME_RUNNING: AtomicBool = AtomicBool::new(false);

// ===== Settings INI =====

/// 获取 settings.ini 路径（exe 同目录）
fn settings_path() -> std::path::PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("settings.ini")
}

/// 从 settings.ini 读取所有 key=value
fn load_settings() -> HashMap<String, String> {
    let mut map = HashMap::new();
    if let Ok(content) = std::fs::read_to_string(settings_path()) {
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') || line.starts_with('[') {
                continue;
            }
            if let Some((key, value)) = line.split_once('=') {
                map.insert(key.trim().to_string(), value.trim().to_string());
            }
        }
    }
    map
}

/// 保存所有 key=value 到 settings.ini
fn save_settings(map: &HashMap<String, String>) {
    let mut lines: Vec<String> = Vec::new();
    lines.push("# nz-rust settings".to_string());
    lines.push(String::new());

    // 按固定顺序输出
    let order = [
        "selected_map",
        "wave_region",
        "gold_region",
        "wave_interval",
        "gold_interval",
        "gold_use_color_filter",
        "gold_color_hex",
        "gold_color_tolerance",
        "ocr_region",
    ];

    for key in &order {
        if let Some(value) = map.get(*key) {
            lines.push(format!("{} = {}", key, value));
        }
    }

    // 写入不在 order 中的其他 key
    for (key, value) in map {
        if !order.contains(&key.as_str()) {
            lines.push(format!("{} = {}", key, value));
        }
    }

    let _ = std::fs::write(settings_path(), lines.join("\n"));
}

// ===== 坐标解析与转换 =====

/// 像素坐标字符串 → 百分比字符串（用于 INI 存储）
/// "3686,1476,3986,1578" → "0.9599,0.6833,1.0380,0.7306"
fn pixel_to_percent(s: &str) -> String {
    let parts: Vec<&str> = s.split(',').collect();
    if parts.len() < 4 {
        return s.to_string();
    }
    let nums: Vec<f64> = parts[..4]
        .iter()
        .filter_map(|p| p.trim().parse().ok())
        .collect();
    if nums.len() < 4 {
        return s.to_string();
    }
    let (sw, sh) = get_screen_resolution();
    let sw = sw as f64;
    let sh = sh as f64;
    format!(
        "{:.6},{:.6},{:.6},{:.6}",
        nums[0] / sw,
        nums[1] / sh,
        nums[2] / sw,
        nums[3] / sh,
    )
}

/// 百分比字符串 → 像素坐标字符串（加载时按当前分辨率转换）
/// "0.9599,0.6833,1.0380,0.7306" → "3686,1476,3986,1578"
fn percent_to_pixel(s: &str) -> String {
    let parts: Vec<&str> = s.split(',').collect();
    if parts.len() < 4 {
        return s.to_string();
    }
    let nums: Vec<f64> = parts[..4]
        .iter()
        .filter_map(|p| p.trim().parse().ok())
        .collect();
    if nums.len() < 4 {
        return s.to_string();
    }
    // 如果值都 <= 1.0，是百分比格式；否则当作像素原样返回
    let is_percent = nums.iter().all(|n| *n <= 1.0001);
    if !is_percent {
        return s.to_string();
    }
    let (sw, sh) = get_screen_resolution();
    let sw = sw as f64;
    let sh = sh as f64;
    format!(
        "{},{},{},{}",
        (nums[0] * sw).round() as i32,
        (nums[1] * sh).round() as i32,
        (nums[2] * sw).round() as i32,
        (nums[3] * sh).round() as i32,
    )
}

/// 解析坐标字符串 "x1,y1,x2,y2" → (x, y, w, h)
/// 也兼容旧格式 "x,y,w,h"（当宽高合理时）
fn parse_region_coords(s: &str) -> Option<(i32, i32, i32, i32)> {
    let parts: Vec<&str> = s.split(',').collect();
    if parts.len() < 4 {
        return None;
    }
    let nums: Vec<i32> = parts[..4]
        .iter()
        .filter_map(|p| p.trim().parse().ok())
        .collect();
    if nums.len() != 4 {
        return None;
    }
    let (x1, y1, x2, y2) = (nums[0], nums[1], nums[2], nums[3]);

    // 判断是 x1,y1,x2,y2（两点）还是 x,y,w,h（旧格式）
    if x2 > x1 && y2 > y1 {
        Some((x1, y1, x2 - x1, y2 - y1))
    } else {
        Some((x1, y1, x2, y2))
    }
}

/// 格式化坐标显示：计算宽高
fn format_region_hint(s: &str) -> String {
    let parts: Vec<&str> = s.split(',').collect();
    if parts.len() < 4 {
        return String::new();
    }
    let nums: Vec<i32> = parts[..4]
        .iter()
        .filter_map(|p| p.trim().parse().ok())
        .collect();
    if nums.len() != 4 {
        return String::new();
    }
    let (x1, y1, x2, y2) = (nums[0], nums[1], nums[2], nums[3]);
    if x2 > x1 && y2 > y1 {
        format!("宽高({},{})", x2 - x1, y2 - y1)
    } else {
        String::new()
    }
}

// ===== 共享日志 =====

struct LogBuffer {
    lines: Vec<String>,
    max_lines: usize,
}

impl LogBuffer {
    fn new(max_lines: usize) -> Self {
        Self {
            lines: Vec::new(),
            max_lines,
        }
    }

    fn push(&mut self, msg: String) {
        self.lines.push(msg);
        if self.lines.len() > self.max_lines {
            self.lines.remove(0);
        }
    }

    fn clear(&mut self) {
        self.lines.clear();
    }
}

// ===== GUI 应用 =====

struct MainApp {
    selected_map: usize,
    log: Arc<Mutex<LogBuffer>>,
    initialized: bool,
    init_error: String,

    // 监控区域配置（x1,y1,x2,y2 格式）
    wave_region: String,
    gold_region: String,
    wave_interval: u64,
    gold_interval: u64,

    // 金币颜色过滤
    gold_use_color_filter: bool,
    gold_color_hex: String,
    gold_color_tolerance: f64,

    // OCR 识别工具
    ocr_region: String,
    ocr_results: Vec<OcrResultItem>,
    ocr_error: String,

    // 设置是否变化（需要保存）
    settings_dirty: bool,
}

impl MainApp {
    /// 从 settings.ini 加载，缺失的用默认值
    fn from_settings() -> Self {
        let s = load_settings();

        Self {
            selected_map: s
                .get("selected_map")
                .and_then(|v| v.parse().ok())
                .unwrap_or(0),
            log: Arc::new(Mutex::new(LogBuffer::new(200))),
            initialized: false,
            init_error: String::new(),

            wave_region: s
                .get("wave_region")
                .map(|v| percent_to_pixel(v))
                .unwrap_or_else(|| "3686,1476,3986,1578".to_string()),
            gold_region: s
                .get("gold_region")
                .map(|v| percent_to_pixel(v))
                .unwrap_or_else(|| "96,112,336,156".to_string()),
            wave_interval: s
                .get("wave_interval")
                .and_then(|v| v.parse().ok())
                .unwrap_or(500),
            gold_interval: s
                .get("gold_interval")
                .and_then(|v| v.parse().ok())
                .unwrap_or(300),

            gold_use_color_filter: s
                .get("gold_use_color_filter")
                .map(|v| v == "true")
                .unwrap_or(false),
            gold_color_hex: s
                .get("gold_color_hex")
                .cloned()
                .unwrap_or_else(|| "d9e1e3".to_string()),
            gold_color_tolerance: s
                .get("gold_color_tolerance")
                .and_then(|v| v.parse().ok())
                .unwrap_or(35.0),

            ocr_region: s
                .get("ocr_region")
                .map(|v| percent_to_pixel(v))
                .unwrap_or_else(|| {
                    let (w, h) = get_screen_resolution();
                    format!("0,0,{},{}", w, h)
                }),
            ocr_results: Vec::new(),
            ocr_error: String::new(),

            settings_dirty: false,
        }
    }

    /// 保存当前设置到 settings.ini（坐标转为百分比存储）
    fn save_settings(&mut self) {
        let mut map = HashMap::new();
        map.insert("selected_map".to_string(), self.selected_map.to_string());
        // 坐标以百分比存储，跨分辨率可移植
        map.insert("wave_region".to_string(), pixel_to_percent(&self.wave_region));
        map.insert("gold_region".to_string(), pixel_to_percent(&self.gold_region));
        map.insert("wave_interval".to_string(), self.wave_interval.to_string());
        map.insert("gold_interval".to_string(), self.gold_interval.to_string());
        map.insert(
            "gold_use_color_filter".to_string(),
            self.gold_use_color_filter.to_string(),
        );
        map.insert("gold_color_hex".to_string(), self.gold_color_hex.clone());
        map.insert(
            "gold_color_tolerance".to_string(),
            self.gold_color_tolerance.to_string(),
        );
        map.insert("ocr_region".to_string(), pixel_to_percent(&self.ocr_region));
        save_settings(&map);
        self.settings_dirty = false;
    }

    /// 解析 hex 颜色 "d9e1e3" → (0xd9, 0xe1, 0xe3)
    fn parse_hex_color(s: &str) -> Option<(u8, u8, u8)> {
        let s = s.trim().trim_start_matches('#');
        if s.len() != 6 {
            return None;
        }
        let r = u8::from_str_radix(&s[0..2], 16).ok()?;
        let g = u8::from_str_radix(&s[2..4], 16).ok()?;
        let b = u8::from_str_radix(&s[4..6], 16).ok()?;
        Some((r, g, b))
    }

    /// 获取当前监控配置（GUI 输入的坐标即实际屏幕坐标，直接使用）
    fn get_monitor_config(&self) -> MonitorConfig {
        let wave_region =
            parse_region_coords(&self.wave_region).unwrap_or((3686, 1476, 300, 102));
        let gold_region =
            parse_region_coords(&self.gold_region).unwrap_or((96, 112, 240, 44));
        let gold_text_color =
            Self::parse_hex_color(&self.gold_color_hex).unwrap_or((0xd9, 0xe1, 0xe3));

        MonitorConfig {
            wave_region,
            gold_region,
            wave_interval_ms: self.wave_interval,
            gold_interval_ms: self.gold_interval,
            gold_text_color,
            gold_color_tolerance: self.gold_color_tolerance,
            gold_use_color_filter: self.gold_use_color_filter,
        }
    }

    fn initialize(&mut self) {
        self.log_msg("正在初始化 OCR 引擎...");
        match ocr::init_ocr() {
            Ok(_) => self.log_msg("OCR 引擎初始化完成"),
            Err(e) => {
                self.init_error = format!("OCR 初始化失败: {}\n请确保 models/ 目录存在", e);
                return;
            }
        }

        self.log_msg("正在初始化输入系统...");
        match input::init(input::InputBackend::Logitech) {
            Ok(_) => self.log_msg("输入系统初始化完成 (Logitech 驱动)"),
            Err(e) => {
                self.log_msg(&format!("Logitech 驱动失败: {}，回退到 SendInput", e));
                let _ = input::init(input::InputBackend::SendInput);
                self.log_msg("输入系统初始化完成 (SendInput)");
            }
        }

        self.initialized = true;
    }

    fn log_msg(&self, msg: &str) {
        if let Ok(mut log) = self.log.lock() {
            let now = chrono_now();
            log.push(format!("[{}] {}", now, msg));
        }
    }

    fn start_game(&self) {
        if GAME_RUNNING.load(Ordering::SeqCst) {
            self.log_msg("游戏正在运行，请先停止");
            return;
        }

        let maps = available_maps();
        if self.selected_map >= maps.len() {
            self.log_msg("未选择有效地图");
            return;
        }

        let map = &maps[self.selected_map];
        let start_fn = map.start_fn;
        let waves_fn = map.waves_fn;
        let map_name = map.name;
        let log = self.log.clone();

        let config = self.get_monitor_config();
        reset_stop();
        monitor::reset_monitors();
        monitor::start_monitors(config);

        GAME_RUNNING.store(true, Ordering::SeqCst);

        thread::spawn(move || {
            log_to(&log, &format!("开始游戏: {}", map_name));

            let mut round = 0;
            const MAX_ROUNDS: i32 = 100;

            while round < MAX_ROUNDS && !should_stop() {
                log_to(&log, &format!("=== 第 {} 轮 ===", round + 1));

                if let Err(e) = start_fn() {
                    log_to(&log, &format!("开始游戏失败: {}", e));
                    if should_stop() {
                        break;
                    }
                }

                if should_stop() {
                    break;
                }

                if let Err(e) = waves_fn() {
                    log_to(&log, &format!("波次执行失败: {}", e));
                    if should_stop() {
                        break;
                    }
                }

                if should_stop() {
                    break;
                }

                round += 1;
                log_to(&log, &format!("第 {} 轮完成", round));
            }

            monitor::stop_monitors();
            GAME_RUNNING.store(false, Ordering::SeqCst);
            log_to(&log, &format!("游戏结束，共完成 {} 轮", round));
        });
    }

    fn stop_game(&self) {
        request_stop();
        monitor::stop_monitors();
        self.log_msg("已请求停止，正在安全退出...");
    }

    fn start_monitor_only(&self) {
        if monitor::is_running() {
            self.log_msg("监控已在运行");
            return;
        }
        let config = self.get_monitor_config();
        monitor::reset_monitors();
        monitor::start_monitors(config);
        self.log_msg("已启动后台监控 (波次 + 金币)");
    }

    fn stop_monitor_only(&self) {
        monitor::stop_monitors();
        self.log_msg("已停止后台监控");
    }

    fn buy_traps_action(&self) {
        let log = self.log.clone();
        thread::spawn(move || {
            log_to(&log, "执行购买陷阱...");
            match buy_traps() {
                Ok(_) => log_to(&log, "购买陷阱完成"),
                Err(e) => log_to(&log, &format!("购买陷阱失败: {}", e)),
            }
        });
    }

    fn run_ocr(&mut self) {
        self.ocr_results.clear();
        self.ocr_error.clear();

        let (x, y, w, h) = match parse_region_coords(&self.ocr_region) {
            Some(r) => r,
            None => {
                self.ocr_error = "区域格式错误，需要 x1,y1,x2,y2".to_string();
                return;
            }
        };

        if w <= 0 || h <= 0 {
            self.ocr_error = "区域无效".to_string();
            return;
        }

        match ocr_screen(x, y, w, h, false, false) {
            Ok(results) => {
                self.log_msg(&format!("OCR 识别到 {} 个文字区域", results.len()));
                for r in &results {
                    let (cx, cy) = r.center();
                    self.log_msg(&format!("  [{}] @ ({}, {})", r.text, cx, cy));
                }
                self.ocr_results = results;
            }
            Err(e) => {
                self.ocr_error = format!("OCR 失败: {}", e);
            }
        }
    }

    /// 带宽高提示的区域输入控件
    fn region_input(ui: &mut egui::Ui, label: &str, value: &mut String, dirty: &mut bool) {
        ui.horizontal(|ui| {
            ui.label(label);
            let resp = ui.add(
                egui::TextEdit::singleline(value).desired_width(180.0),
            );
            if resp.changed() {
                *dirty = true;
            }
            let hint = format_region_hint(value);
            if !hint.is_empty() {
                ui.colored_label(egui::Color32::from_rgb(120, 180, 120), &hint);
            }
        });
    }
}

/// 向共享日志写入消息
fn log_to(log: &Arc<Mutex<LogBuffer>>, msg: &str) {
    if let Ok(mut log) = log.lock() {
        let now = chrono_now();
        log.push(format!("[{}] {}", now, msg));
    }
}

fn chrono_now() -> String {
    let dur = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = dur.as_secs() % 86400;
    let hours = (secs / 3600 + 8) % 24;
    let mins = (secs % 3600) / 60;
    let s = secs % 60;
    format!("{:02}:{:02}:{:02}", hours, mins, s)
}

fn format_gold(gold: i64) -> String {
    let s = gold.to_string();
    let mut result = String::new();
    for (i, ch) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(ch);
    }
    result.chars().rev().collect()
}

impl eframe::App for MainApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if !self.initialized && self.init_error.is_empty() {
            self.initialize();
        }

        ctx.request_repaint_after(std::time::Duration::from_millis(200));

        // 处理全局热键
        let hotkey = HOTKEY_EVENT.swap(0, Ordering::SeqCst);
        if hotkey == 1 && self.initialized && self.init_error.is_empty() {
            self.start_game();
        } else if hotkey == 2 {
            self.stop_game();
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("逆战：未来 - 自动化工具");
                ui.add_space(20.0);
                let (sw, sh) = get_screen_resolution();
                let (sx, _) = get_scale_factors();
                if (sx - 1.0).abs() > 0.01 {
                    ui.colored_label(
                        egui::Color32::from_rgb(100, 180, 255),
                        format!("{}x{} (×{:.0})", sw, sh, sx),
                    );
                } else {
                    ui.label(format!("{}x{}", sw, sh));
                }
            });
            ui.separator();

            if !self.init_error.is_empty() {
                ui.colored_label(egui::Color32::RED, &self.init_error);
                ui.separator();
                return;
            }

            // ===== 控制区域 =====
            ui.horizontal(|ui| {
                ui.label("地图:");
                let maps = available_maps();
                let current_name = if self.selected_map < maps.len() {
                    format!(
                        "{} ({})",
                        maps[self.selected_map].name, maps[self.selected_map].difficulty
                    )
                } else {
                    "无".to_string()
                };
                let old_map = self.selected_map;
                egui::ComboBox::from_id_salt("map_select")
                    .selected_text(&current_name)
                    .show_ui(ui, |ui| {
                        for (i, map) in maps.iter().enumerate() {
                            let label = format!("{} ({})", map.name, map.difficulty);
                            ui.selectable_value(&mut self.selected_map, i, label);
                        }
                    });
                if self.selected_map != old_map {
                    self.settings_dirty = true;
                }

                ui.add_space(20.0);

                let is_running = GAME_RUNNING.load(Ordering::SeqCst);
                if is_running {
                    if ui.button("停止 (F2)").clicked() {
                        self.stop_game();
                    }
                } else {
                    if ui.button("启动 (F1)").clicked() {
                        self.start_game();
                    }
                }

                ui.add_space(10.0);
                if is_running {
                    ui.colored_label(egui::Color32::GREEN, "运行中");
                } else {
                    ui.colored_label(egui::Color32::GRAY, "已停止");
                }
            });

            ui.separator();

            // ===== 实时状态 + 监控控制 =====
            ui.horizontal(|ui| {
                let wave = monitor::current_wave();
                let gold = monitor::current_gold();
                let monitor_running = monitor::is_running();

                ui.label(format!(
                    "波次: {}",
                    if wave == 0 {
                        "未开始".to_string()
                    } else {
                        wave.to_string()
                    }
                ));
                ui.add_space(10.0);
                ui.label(format!(
                    "金币: {}",
                    if gold == 0 {
                        "-".to_string()
                    } else {
                        format_gold(gold)
                    }
                ));
                ui.add_space(10.0);

                if monitor_running {
                    ui.colored_label(egui::Color32::from_rgb(100, 200, 100), "监控中");
                    if ui.small_button("停止监控").clicked() {
                        self.stop_monitor_only();
                    }
                } else {
                    ui.colored_label(egui::Color32::GRAY, "监控未启动");
                    if ui.small_button("启动监控").clicked() {
                        self.start_monitor_only();
                    }
                }
            });

            ui.separator();

            // ===== 监控区域配置 =====
            egui::CollapsingHeader::new("监控区域配置")
                .default_open(false)
                .show(ui, |ui| {
                    Self::region_input(
                        ui,
                        "波次区域 (x1,y1,x2,y2):",
                        &mut self.wave_region,
                        &mut self.settings_dirty,
                    );
                    Self::region_input(
                        ui,
                        "金币区域 (x1,y1,x2,y2):",
                        &mut self.gold_region,
                        &mut self.settings_dirty,
                    );
                    ui.horizontal(|ui| {
                        ui.label("波次间隔(ms):");
                        let old_wi = self.wave_interval;
                        ui.add(
                            egui::DragValue::new(&mut self.wave_interval).range(100..=5000),
                        );
                        if self.wave_interval != old_wi {
                            self.settings_dirty = true;
                        }
                        ui.add_space(10.0);
                        ui.label("金币间隔(ms):");
                        let old_gi = self.gold_interval;
                        ui.add(
                            egui::DragValue::new(&mut self.gold_interval).range(100..=5000),
                        );
                        if self.gold_interval != old_gi {
                            self.settings_dirty = true;
                        }
                    });
                    ui.separator();
                    ui.horizontal(|ui| {
                        let old_cf = self.gold_use_color_filter;
                        ui.checkbox(&mut self.gold_use_color_filter, "金币颜色过滤");
                        if self.gold_use_color_filter != old_cf {
                            self.settings_dirty = true;
                        }
                        if self.gold_use_color_filter {
                            ui.label("颜色 #");
                            let resp = ui.add(
                                egui::TextEdit::singleline(&mut self.gold_color_hex)
                                    .desired_width(60.0),
                            );
                            if resp.changed() {
                                self.settings_dirty = true;
                            }
                            ui.label("容差:");
                            let old_tol = self.gold_color_tolerance;
                            ui.add(
                                egui::DragValue::new(&mut self.gold_color_tolerance)
                                    .range(10.0..=100.0)
                                    .speed(1.0),
                            );
                            if self.gold_color_tolerance != old_tol {
                                self.settings_dirty = true;
                            }
                        }
                    });
                    if self.gold_use_color_filter {
                        if let Some((r, g, b)) = Self::parse_hex_color(&self.gold_color_hex) {
                            let color = egui::Color32::from_rgb(r, g, b);
                            ui.horizontal(|ui| {
                                ui.label("预览:");
                                let (rect, _) = ui.allocate_exact_size(
                                    egui::vec2(40.0, 14.0),
                                    egui::Sense::hover(),
                                );
                                ui.painter().rect_filled(rect, 2.0, color);
                                ui.label("(关闭则使用 Otsu 二值化)");
                            });
                        }
                    }
                });

            ui.separator();

            // ===== 快捷操作 =====
            ui.horizontal(|ui| {
                if ui.button("购买陷阱").clicked() {
                    self.buy_traps_action();
                }
            });

            ui.separator();

            // ===== OCR 识别工具 =====
            egui::CollapsingHeader::new("OCR 识别工具")
                .default_open(false)
                .show(ui, |ui| {
                    Self::region_input(
                        ui,
                        "区域 (x1,y1,x2,y2):",
                        &mut self.ocr_region,
                        &mut self.settings_dirty,
                    );
                    ui.horizontal(|ui| {
                        if ui.button("全屏").clicked() {
                            let (w, h) = get_screen_resolution();
                            self.ocr_region = format!("0,0,{},{}", w, h);
                        }
                        if ui.button("右半屏").clicked() {
                            let (w, h) = get_screen_resolution();
                            self.ocr_region = format!("{},0,{},{}", w / 2, w, h);
                        }
                        if ui.button("执行 OCR").clicked() {
                            self.run_ocr();
                        }
                    });

                    if !self.ocr_error.is_empty() {
                        ui.colored_label(egui::Color32::RED, &self.ocr_error);
                    }

                    if !self.ocr_results.is_empty() {
                        ui.label(format!(
                            "识别到 {} 个结果 (点击可移动鼠标并点击):",
                            self.ocr_results.len()
                        ));
                        let mut click_target: Option<(i32, i32)> = None;
                        egui::ScrollArea::vertical()
                            .id_salt("ocr_results_scroll")
                            .max_height(120.0)
                            .show(ui, |ui| {
                                for r in &self.ocr_results {
                                    let (cx, cy) = r.center();
                                    let label =
                                        format!("[{}] @ ({}, {})", r.text, cx, cy);
                                    if ui.button(&label).clicked() {
                                        click_target = Some((cx, cy));
                                    }
                                }
                            });
                        if let Some((x, y)) = click_target {
                            click_at(x, y);
                            self.log_msg(&format!("点击了 ({}, {})", x, y));
                        }
                    }
                });

            ui.separator();

            // ===== 日志面板 =====
            ui.horizontal(|ui| {
                ui.label("日志");
                if ui.small_button("清空").clicked() {
                    if let Ok(mut log) = self.log.lock() {
                        log.clear();
                    }
                }
            });

            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    if let Ok(log) = self.log.lock() {
                        for line in &log.lines {
                            ui.label(line);
                        }
                    }
                });
        });

        // 设置变化时自动保存
        if self.settings_dirty {
            self.save_settings();
        }
    }
}

// ===== 全局热键 =====

fn start_hotkey_thread() {
    thread::spawn(|| {
        use windows::Win32::Foundation::HWND;
        use windows::Win32::UI::Input::KeyboardAndMouse::{
            RegisterHotKey, HOT_KEY_MODIFIERS, VK_F1, VK_F2,
        };
        use windows::Win32::UI::WindowsAndMessaging::{GetMessageW, MSG, WM_HOTKEY};

        const HOTKEY_F1: i32 = 1;
        const HOTKEY_F2: i32 = 2;

        unsafe {
            let _ = RegisterHotKey(
                HWND::default(),
                HOTKEY_F1,
                HOT_KEY_MODIFIERS(0),
                VK_F1.0 as u32,
            );
            let _ = RegisterHotKey(
                HWND::default(),
                HOTKEY_F2,
                HOT_KEY_MODIFIERS(0),
                VK_F2.0 as u32,
            );
        }

        println!("[Hotkey] 全局热键已注册: F1=启动, F2=停止");

        loop {
            let mut msg = MSG::default();
            unsafe {
                if GetMessageW(&mut msg, HWND::default(), 0, 0).as_bool() {
                    if msg.message == WM_HOTKEY {
                        match msg.wParam.0 as i32 {
                            HOTKEY_F1 => {
                                println!("[Hotkey] F1 按下 → 启动");
                                HOTKEY_EVENT.store(1, Ordering::SeqCst);
                            }
                            HOTKEY_F2 => {
                                println!("[Hotkey] F2 按下 → 停止");
                                HOTKEY_EVENT.store(2, Ordering::SeqCst);
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    });
}

/// 检查是否以管理员权限运行
fn is_elevated() -> bool {
    use windows::Win32::Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY};
    use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcessToken};

    unsafe {
        let mut token = windows::Win32::Foundation::HANDLE::default();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token).is_err() {
            return false;
        }
        let mut elevation = TOKEN_ELEVATION::default();
        let mut size = 0u32;
        let ok = GetTokenInformation(
            token,
            TokenElevation,
            Some(&mut elevation as *mut _ as *mut _),
            std::mem::size_of::<TOKEN_ELEVATION>() as u32,
            &mut size,
        );
        let _ = windows::Win32::Foundation::CloseHandle(token);
        ok.is_ok() && elevation.TokenIsElevated != 0
    }
}

/// 以管理员权限重新启动自身
fn relaunch_as_admin() -> bool {
    use windows::Win32::UI::Shell::ShellExecuteW;
    use windows::core::PCWSTR;

    let exe = std::env::current_exe().unwrap_or_default();
    let exe_wide: Vec<u16> = exe.to_string_lossy().encode_utf16().chain(std::iter::once(0)).collect();
    let verb: Vec<u16> = "runas\0".encode_utf16().collect();

    unsafe {
        let result = ShellExecuteW(
            None,
            PCWSTR(verb.as_ptr()),
            PCWSTR(exe_wide.as_ptr()),
            PCWSTR::null(),
            PCWSTR::null(),
            windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL,
        );
        result.0 as usize > 32
    }
}

fn main() -> eframe::Result<()> {
    // 自动提权：如果不是管理员权限，则以管理员身份重新启动
    if !is_elevated() {
        if relaunch_as_admin() {
            // 成功启动了提权后的新进程，退出当前进程
            std::process::exit(0);
        }
        // 用户拒绝了 UAC 提示或提权失败，继续以普通权限运行
    }

    start_hotkey_thread();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([600.0, 500.0])
            .with_title("逆战：未来 - 自动化工具"),
        ..Default::default()
    };

    eframe::run_native(
        "nz-rust",
        options,
        Box::new(|cc| {
            let mut fonts = egui::FontDefinitions::default();
            if let Ok(font_data) = std::fs::read("C:\\Windows\\Fonts\\msyh.ttc") {
                fonts.font_data.insert(
                    "msyh".to_owned(),
                    egui::FontData::from_owned(font_data).into(),
                );
                fonts
                    .families
                    .entry(egui::FontFamily::Proportional)
                    .or_default()
                    .insert(0, "msyh".to_owned());
                fonts
                    .families
                    .entry(egui::FontFamily::Monospace)
                    .or_default()
                    .insert(0, "msyh".to_owned());
            }
            cc.egui_ctx.set_fonts(fonts);

            Ok(Box::new(MainApp::from_settings()))
        }),
    )
}
