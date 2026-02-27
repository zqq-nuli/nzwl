//! OCR 测试工具 - GUI 版本
//!
//! 用于测试指定屏幕区域的 OCR 识别效果
//! 以及测试键盘鼠标输入

use eframe::egui;
use std::sync::{Arc, Mutex};
use std::thread;

// 导入主项目的模块
use nz_rust::input::{
    self, get_vk_code, key_down, key_up, left_click, move_to, press_key, send_relative, tap_key,
    InputBackend,
};
use nz_rust::ocr::{init_ocr, ocr_screen, ocr_screen_small, OcrResultItem};
use nz_rust::screen::capture_region;

/// 测试动作类型
#[derive(Clone)]
enum TestAction {
    /// 移动鼠标到坐标 (x, y)
    MoveTo(i32, i32),
    /// 单击左键
    Click,
    /// 单击键盘某键
    TapKey(String),
    /// 按住键盘某键若干秒
    HoldKey(String, f64),
    /// 转动视角（度数，正数向右，负数向左）
    TurnView(f64),
    /// 按下键（不松开）
    KeyDown(String),
    /// 松开键
    KeyUp(String),
}

/// 1度对应的鼠标移动像素 (实测 360度 = 4474像素)
const PIXELS_PER_DEGREE: f64 = 12.43;

impl TestAction {
    /// 显示文本
    fn display(&self) -> String {
        match self {
            TestAction::MoveTo(x, y) => format!("移动鼠标到 ({}, {})", x, y),
            TestAction::Click => "单击左键".to_string(),
            TestAction::TapKey(key) => format!("单击 {} 键", key),
            TestAction::HoldKey(key, secs) => format!("按住 {} 键 {} 秒", key, secs),
            TestAction::TurnView(degrees) => {
                if *degrees >= 0.0 {
                    format!("向右转 {} 度", degrees)
                } else {
                    format!("向左转 {} 度", degrees.abs())
                }
            }
            TestAction::KeyDown(key) => format!("{} 按下", key),
            TestAction::KeyUp(key) => format!("{} 弹起", key),
        }
    }

    /// 生成 Rust 代码
    fn to_code(&self, interval_ms: u64) -> String {
        match self {
            TestAction::MoveTo(x, y) => {
                format!(
                    "move_to({}, {});\nthread::sleep(Duration::from_millis({}));",
                    x, y, interval_ms
                )
            }
            TestAction::Click => {
                format!(
                    "left_click();\nthread::sleep(Duration::from_millis({}));",
                    interval_ms
                )
            }
            TestAction::TapKey(key) => {
                format!(
                    "tap_key(VK_{});\nthread::sleep(Duration::from_millis({}));",
                    key, interval_ms
                )
            }
            TestAction::HoldKey(key, secs) => {
                format!(
                    "press_key(VK_{}, {});\nthread::sleep(Duration::from_millis({}));",
                    key, secs, interval_ms
                )
            }
            TestAction::TurnView(degrees) => {
                let pixels = (*degrees * PIXELS_PER_DEGREE) as i32;
                format!(
                    "send_relative({}, 0); // 转 {} 度\nthread::sleep(Duration::from_millis({}));",
                    pixels, degrees, interval_ms
                )
            }
            TestAction::KeyDown(key) => {
                format!(
                    "key_down(VK_{});\nthread::sleep(Duration::from_millis({}));",
                    key, interval_ms
                )
            }
            TestAction::KeyUp(key) => {
                format!(
                    "key_up(VK_{});\nthread::sleep(Duration::from_millis({}));",
                    key, interval_ms
                )
            }
        }
    }
}

fn main() -> eframe::Result<()> {
    // 初始化 OCR 引擎
    println!("正在初始化 OCR 引擎...");
    if let Err(e) = init_ocr() {
        eprintln!("OCR 初始化失败: {}", e);
        return Ok(());
    }
    println!("OCR 引擎初始化完成");

    // 初始化输入系统 - 尝试使用 Logitech 驱动
    println!("正在初始化输入系统...");
    match input::init(InputBackend::Logitech) {
        Ok(_) => println!("输入系统初始化完成 (Logitech 驱动)"),
        Err(e) => {
            println!("Logitech 驱动初始化失败: {}, 回退到 SendInput", e);
            let _ = input::init(InputBackend::SendInput);
        }
    }

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([600.0, 900.0])
            .with_always_on_top(),
        ..Default::default()
    };

    eframe::run_native(
        "OCR 测试工具",
        options,
        Box::new(|cc| {
            // 加载中文字体
            let mut fonts = egui::FontDefinitions::default();

            // 尝试加载 Windows 系统中文字体
            if let Ok(font_data) = std::fs::read("C:\\Windows\\Fonts\\msyh.ttc") {
                fonts.font_data.insert(
                    "msyh".to_owned(),
                    egui::FontData::from_owned(font_data).into(),
                );

                // 将中文字体添加到所有字体族的首位
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

            Ok(Box::new(OcrTestApp::default()))
        }),
    )
}

struct OcrTestApp {
    // 区域坐标 (格式: x1,y1,x2,y2 或 x1,y1,x2,y2,其他内容)
    coords_input: String,

    // OCR 结果
    results: Vec<OcrResultItem>,
    ocr_time_ms: f64,
    error_msg: String,
    copy_msg: String,

    // 预览图像
    preview_texture: Option<egui::TextureHandle>,

    // ===== 小区域预处理 =====
    use_preprocess: bool,
    preprocess_scale: u32,

    // ===== 键盘鼠标测试 =====
    // 动作输入
    action_input: String,
    // 动作配置列表
    action_configs: Vec<TestAction>,
    // 测试状态消息
    action_msg: String,
    // 是否正在执行
    is_running: Arc<Mutex<bool>>,
    // 执行间隔（秒）
    action_interval: f64,
}

impl Default for OcrTestApp {
    fn default() -> Self {
        Self {
            coords_input: "0,0,400,300".to_string(),
            results: Vec::new(),
            ocr_time_ms: 0.0,
            error_msg: String::new(),
            copy_msg: String::new(),
            preview_texture: None,
            // 小区域预处理
            use_preprocess: false,
            preprocess_scale: 3,
            // 键盘鼠标测试
            action_input: String::new(),
            action_configs: Vec::new(),
            action_msg: String::new(),
            is_running: Arc::new(Mutex::new(false)),
            action_interval: 0.5,
        }
    }
}

impl OcrTestApp {
    /// 解析坐标输入，返回 (start_x, start_y, end_x, end_y)
    /// 格式: "x1,y1,x2,y2" 或 "x1,y1,x2,y2,其他内容"
    fn parse_coords(&self) -> Result<(i32, i32, i32, i32), String> {
        let parts: Vec<&str> = self.coords_input.split(',').collect();

        if parts.len() < 4 {
            return Err("坐标格式错误，需要至少4个数字: x1,y1,x2,y2".to_string());
        }

        let start_x: i32 = parts[0].trim().parse()
            .map_err(|_| format!("第1个坐标 '{}' 无效", parts[0].trim()))?;
        let start_y: i32 = parts[1].trim().parse()
            .map_err(|_| format!("第2个坐标 '{}' 无效", parts[1].trim()))?;
        let end_x: i32 = parts[2].trim().parse()
            .map_err(|_| format!("第3个坐标 '{}' 无效", parts[2].trim()))?;
        let end_y: i32 = parts[3].trim().parse()
            .map_err(|_| format!("第4个坐标 '{}' 无效", parts[3].trim()))?;

        Ok((start_x, start_y, end_x, end_y))
    }

    /// 生成并复制 OCR 代码到剪贴板
    fn copy_code(&mut self, ui: &mut egui::Ui) {
        self.error_msg.clear();
        self.copy_msg.clear();

        // 解析坐标
        let (start_x, start_y, end_x, end_y) = match self.parse_coords() {
            Ok(coords) => coords,
            Err(e) => {
                self.error_msg = e;
                return;
            }
        };

        // 计算宽高
        let width = end_x - start_x;
        let height = end_y - start_y;

        if width <= 0 || height <= 0 {
            self.error_msg = "区域无效：结束坐标必须大于起始坐标".to_string();
            return;
        }

        // 生成代码
        let code = format!(
            "let results = ocr_screen({}, {}, {}, {}, false, IS_DEBUG)?;",
            start_x, start_y, width, height
        );

        // 复制到剪贴板
        ui.ctx().copy_text(code.clone());
        self.copy_msg = format!("已复制: {}", code);
    }

    /// 添加移动鼠标动作
    fn add_move_action(&mut self, input: &str) {
        self.action_msg.clear();
        let parts: Vec<&str> = input.split(',').collect();
        if parts.len() < 2 {
            self.action_msg = "格式错误，需要: x,y".to_string();
            return;
        }
        let x: i32 = match parts[0].trim().parse() {
            Ok(v) => v,
            Err(_) => {
                self.action_msg = format!("X坐标 '{}' 无效", parts[0].trim());
                return;
            }
        };
        let y: i32 = match parts[1].trim().parse() {
            Ok(v) => v,
            Err(_) => {
                self.action_msg = format!("Y坐标 '{}' 无效", parts[1].trim());
                return;
            }
        };
        self.action_configs.push(TestAction::MoveTo(x, y));
        self.action_msg = format!("已添加: 移动鼠标到 ({}, {})", x, y);
    }

    /// 添加单击左键动作
    fn add_click_action(&mut self) {
        self.action_msg.clear();
        self.action_configs.push(TestAction::Click);
        self.action_msg = "已添加: 单击左键".to_string();
    }

    /// 添加单击键盘动作
    fn add_tap_action(&mut self, key: &str) {
        self.action_msg.clear();
        let key = key.trim().to_uppercase();
        if key.is_empty() {
            self.action_msg = "请输入键名".to_string();
            return;
        }
        if get_vk_code(&key).is_none() {
            self.action_msg = format!("未知的键名 '{}'. 支持: A-Z, 0-9, SPACE, ENTER, ESC, TAB, SHIFT, CTRL, ALT, F1, F2", key);
            return;
        }
        self.action_configs.push(TestAction::TapKey(key.clone()));
        self.action_msg = format!("已添加: 单击 {} 键", key);
    }

    /// 添加按住键盘动作
    fn add_hold_action(&mut self, input: &str) {
        self.action_msg.clear();
        let parts: Vec<&str> = input.split(',').collect();
        if parts.len() < 2 {
            self.action_msg = "格式错误，需要: 键名,秒数 (如 w,3)".to_string();
            return;
        }
        let key = parts[0].trim().to_uppercase();
        let seconds: f64 = match parts[1].trim().parse() {
            Ok(v) => v,
            Err(_) => {
                self.action_msg = format!("秒数 '{}' 无效", parts[1].trim());
                return;
            }
        };
        if get_vk_code(&key).is_none() {
            self.action_msg = format!("未知的键名 '{}'. 支持: A-Z, 0-9, SPACE, ENTER, ESC, TAB, SHIFT, CTRL, ALT, F1, F2", key);
            return;
        }
        self.action_configs.push(TestAction::HoldKey(key.clone(), seconds));
        self.action_msg = format!("已添加: 按住 {} 键 {} 秒", key, seconds);
    }

    /// 添加转动视角动作
    fn add_turn_action(&mut self, input: &str) {
        self.action_msg.clear();
        let degrees: f64 = match input.trim().parse() {
            Ok(v) => v,
            Err(_) => {
                self.action_msg = format!("度数 '{}' 无效，请输入数字（正数向右，负数向左）", input.trim());
                return;
            }
        };
        self.action_configs.push(TestAction::TurnView(degrees));
        if degrees >= 0.0 {
            self.action_msg = format!("已添加: 向右转 {} 度", degrees);
        } else {
            self.action_msg = format!("已添加: 向左转 {} 度", degrees.abs());
        }
    }

    /// 添加按下键动作（不松开）
    fn add_keydown_action(&mut self, key: &str) {
        self.action_msg.clear();
        let key = key.trim().to_uppercase();
        if key.is_empty() {
            self.action_msg = "请输入键名".to_string();
            return;
        }
        if get_vk_code(&key).is_none() {
            self.action_msg = format!("未知的键名 '{}'", key);
            return;
        }
        self.action_configs.push(TestAction::KeyDown(key.clone()));
        self.action_msg = format!("已添加: {} 按下", key);
    }

    /// 添加松开键动作
    fn add_keyup_action(&mut self, key: &str) {
        self.action_msg.clear();
        let key = key.trim().to_uppercase();
        if key.is_empty() {
            self.action_msg = "请输入键名".to_string();
            return;
        }
        if get_vk_code(&key).is_none() {
            self.action_msg = format!("未知的键名 '{}'", key);
            return;
        }
        self.action_configs.push(TestAction::KeyUp(key.clone()));
        self.action_msg = format!("已添加: {} 弹起", key);
    }

    /// 复制动作代码到剪贴板
    fn copy_action_code(&mut self, ui: &mut egui::Ui) {
        self.action_msg.clear();
        if self.action_configs.is_empty() {
            self.action_msg = "请先添加动作配置".to_string();
            return;
        }

        let interval_ms = (self.action_interval * 1000.0) as u64;
        let mut code_lines: Vec<String> = Vec::new();

        for action in &self.action_configs {
            code_lines.push(action.to_code(interval_ms));
        }

        let code = code_lines.join("\n");
        ui.ctx().copy_text(code.clone());
        self.action_msg = "代码已复制到剪贴板".to_string();
    }

    /// 执行测试
    fn run_action_test(&mut self) {
        // 检查是否正在运行
        {
            let mut running = self.is_running.lock().unwrap();
            if *running {
                self.action_msg = "正在执行中，请等待...".to_string();
                return;
            }
            *running = true;
        }

        if self.action_configs.is_empty() {
            self.action_msg = "请先添加动作配置".to_string();
            *self.is_running.lock().unwrap() = false;
            return;
        }

        self.action_msg = "3秒后开始执行...".to_string();

        // 复制配置到线程
        let configs = self.action_configs.clone();
        let is_running = Arc::clone(&self.is_running);
        let interval = self.action_interval;

        // 在后台线程执行
        thread::spawn(move || {
            // 等待3秒让用户切换窗口
            thread::sleep(std::time::Duration::from_secs(3));

            println!("[动作测试] 开始执行，间隔 {} 秒", interval);

            // 依次执行动作
            for (i, action) in configs.iter().enumerate() {
                println!("[动作测试] {}: {}", i + 1, action.display());

                match action {
                    TestAction::MoveTo(x, y) => {
                        move_to(*x, *y);
                    }
                    TestAction::Click => {
                        left_click();
                    }
                    TestAction::TapKey(key) => {
                        if let Some(vk) = get_vk_code(key) {
                            tap_key(vk);
                        }
                    }
                    TestAction::HoldKey(key, secs) => {
                        if let Some(vk) = get_vk_code(key) {
                            press_key(vk, *secs);
                        }
                    }
                    TestAction::TurnView(degrees) => {
                        let pixels = (degrees * PIXELS_PER_DEGREE) as i32;
                        send_relative(pixels, 0);
                    }
                    TestAction::KeyDown(key) => {
                        if let Some(vk) = get_vk_code(key) {
                            key_down(vk);
                        }
                    }
                    TestAction::KeyUp(key) => {
                        if let Some(vk) = get_vk_code(key) {
                            key_up(vk);
                        }
                    }
                }

                // 执行间隔
                if i < configs.len() - 1 {
                    thread::sleep(std::time::Duration::from_secs_f64(interval));
                }
            }

            println!("[动作测试] 执行完成");
            *is_running.lock().unwrap() = false;
        });
    }

    fn run_ocr(&mut self, ctx: &egui::Context) {
        self.error_msg.clear();
        self.results.clear();

        // 解析坐标
        let (start_x, start_y, end_x, end_y) = match self.parse_coords() {
            Ok(coords) => coords,
            Err(e) => {
                self.error_msg = e;
                return;
            }
        };

        // 计算宽高
        let width = end_x - start_x;
        let height = end_y - start_y;

        if width <= 0 || height <= 0 {
            self.error_msg = "区域无效：结束坐标必须大于起始坐标".to_string();
            return;
        }

        // 截图并更新预览
        match capture_region(start_x, start_y, width, height) {
            Ok(img) => {
                // 转换为 egui 可用的格式
                let size = [img.width() as usize, img.height() as usize];
                let pixels: Vec<egui::Color32> = img
                    .pixels()
                    .map(|p| egui::Color32::from_rgb(p[0], p[1], p[2]))
                    .collect();

                let color_image = egui::ColorImage { size, pixels };
                self.preview_texture = Some(ctx.load_texture(
                    "preview",
                    color_image,
                    egui::TextureOptions::default(),
                ));
            }
            Err(e) => {
                self.error_msg = format!("截图失败: {}", e);
                return;
            }
        }

        // 执行 OCR
        let start_time = std::time::Instant::now();
        let ocr_result = if self.use_preprocess {
            ocr_screen_small(start_x, start_y, width, height, self.preprocess_scale, true)
        } else {
            ocr_screen(start_x, start_y, width, height, false, false)
        };
        match ocr_result {
            Ok(results) => {
                self.ocr_time_ms = start_time.elapsed().as_secs_f64() * 1000.0;
                self.results = results;
            }
            Err(e) => {
                self.error_msg = format!("OCR 失败: {}", e);
            }
        }
    }

}

impl eframe::App for OcrTestApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("OCR 测试工具");
            ui.separator();

            // 坐标输入区域
            ui.horizontal(|ui| {
                ui.label("坐标 (x1,y1,x2,y2):");
                ui.add(egui::TextEdit::singleline(&mut self.coords_input).desired_width(300.0));
            });

            ui.add_space(10.0);

            // 快捷设置按钮
            ui.horizontal(|ui| {
                if ui.button("全屏 1920x1080").clicked() {
                    self.coords_input = "0,0,1920,1080".to_string();
                }
                if ui.button("左上角 400x300").clicked() {
                    self.coords_input = "0,0,400,300".to_string();
                }
                if ui.button("中心区域").clicked() {
                    self.coords_input = "560,340,1360,740".to_string();
                }
            });

            ui.add_space(5.0);

            // 小区域预处理选项
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.use_preprocess, "小区域预处理 (放大+二值化)");
                if self.use_preprocess {
                    ui.label("放大倍数:");
                    ui.add(egui::DragValue::new(&mut self.preprocess_scale).range(2..=6));
                }
            });

            ui.add_space(5.0);

            // 执行按钮和复制代码按钮
            ui.horizontal(|ui| {
                if ui.button("执行 OCR").clicked() {
                    self.run_ocr(ctx);
                    self.copy_msg.clear();
                }

                ui.add_space(10.0);

                if ui.button("复制代码").clicked() {
                    self.copy_code(ui);
                }

                // 显示复制成功提示
                if !self.copy_msg.is_empty() {
                    ui.colored_label(egui::Color32::GREEN, &self.copy_msg);
                }
            });

            ui.separator();

            // 错误信息
            if !self.error_msg.is_empty() {
                ui.colored_label(egui::Color32::RED, &self.error_msg);
            }

            // OCR 结果统计
            if !self.results.is_empty() || self.ocr_time_ms > 0.0 {
                ui.label(format!(
                    "识别到 {} 个文字区域 | 耗时: {:.1}ms",
                    self.results.len(),
                    self.ocr_time_ms
                ));
            }

            ui.separator();

            // 预览图像和结果的分栏显示
            ui.horizontal(|ui| {
                // 左侧：预览图像
                ui.vertical(|ui| {
                    ui.label("截图预览:");
                    if let Some(texture) = &self.preview_texture {
                        let max_size = egui::vec2(280.0, 200.0);
                        let img_size = texture.size_vec2();
                        let scale = (max_size.x / img_size.x).min(max_size.y / img_size.y).min(1.0);
                        let display_size = img_size * scale;
                        ui.image((texture.id(), display_size));
                    } else {
                        ui.label("(执行 OCR 后显示预览)");
                    }
                });

                ui.separator();

                // 右侧：OCR 结果
                ui.vertical(|ui| {
                    ui.label("识别结果:");
                    egui::ScrollArea::vertical()
                        .max_height(180.0)
                        .show(ui, |ui| {
                            if self.results.is_empty() {
                                ui.label("(无结果)");
                            } else {
                                for (i, r) in self.results.iter().enumerate() {
                                    ui.horizontal(|ui| {
                                        ui.label(format!("{}.", i + 1));
                                        ui.monospace(&r.text);
                                    });
                                }
                            }
                        });
                });
            });

            ui.separator();

            // 详细结果表格
            ui.label("详细信息:");
            egui::ScrollArea::vertical()
                .max_height(150.0)
                .show(ui, |ui| {
                    egui::Grid::new("results_grid")
                        .striped(true)
                        .min_col_width(50.0)
                        .show(ui, |ui| {
                            // 表头
                            ui.label("序号");
                            ui.label("文字");
                            ui.label("中心坐标");
                            ui.label("置信度");
                            ui.end_row();

                            // 数据行
                            for (i, r) in self.results.iter().enumerate() {
                                let (cx, cy) = r.center();
                                ui.label(format!("{}", i + 1));
                                ui.label(&r.text);
                                ui.label(format!("({}, {})", cx, cy));
                                ui.label(format!("{:.2}", r.score));
                                ui.end_row();
                            }
                        });
                });

            ui.separator();

            // ===== 键盘鼠标测试区域 =====
            ui.heading("键盘鼠标测试");

            ui.add_space(5.0);

            // 输入框
            ui.horizontal(|ui| {
                ui.label("参数输入:");
                ui.add(egui::TextEdit::singleline(&mut self.action_input).desired_width(150.0));
            });

            // 动作按钮 - 第一行
            ui.horizontal(|ui| {
                if ui.button("移动鼠标 (x,y)").clicked() {
                    let input = self.action_input.clone();
                    self.add_move_action(&input);
                }
                if ui.button("单击左键").clicked() {
                    self.add_click_action();
                }
                if ui.button("单击键 (键名)").clicked() {
                    let input = self.action_input.clone();
                    self.add_tap_action(&input);
                }
            });

            // 动作按钮 - 第二行
            ui.horizontal(|ui| {
                if ui.button("按住键 (键名,秒)").clicked() {
                    let input = self.action_input.clone();
                    self.add_hold_action(&input);
                }
                if ui.button("转视角 (度数)").clicked() {
                    let input = self.action_input.clone();
                    self.add_turn_action(&input);
                }
                ui.label("正数右，负数左");
            });

            // 动作按钮 - 第三行（组合键）
            ui.horizontal(|ui| {
                if ui.button("按下键 (键名)").clicked() {
                    let input = self.action_input.clone();
                    self.add_keydown_action(&input);
                }
                if ui.button("弹起键 (键名)").clicked() {
                    let input = self.action_input.clone();
                    self.add_keyup_action(&input);
                }
                ui.label("用于组合键");
            });

            // 间隔设置
            ui.horizontal(|ui| {
                ui.label("执行间隔(秒):");
                ui.add(egui::DragValue::new(&mut self.action_interval)
                    .range(0.0..=10.0)
                    .speed(0.1));
            });

            // 状态消息
            if !self.action_msg.is_empty() {
                let color = if self.action_msg.contains("错误") || self.action_msg.contains("未知") || self.action_msg.contains("请") {
                    egui::Color32::RED
                } else {
                    egui::Color32::GREEN
                };
                ui.colored_label(color, &self.action_msg);
            }

            ui.add_space(5.0);

            // 动作配置列表
            ui.label("动作列表:");
            egui::ScrollArea::vertical()
                .id_salt("action_configs_scroll")
                .max_height(100.0)
                .show(ui, |ui| {
                    let mut to_remove: Option<usize> = None;
                    for (i, action) in self.action_configs.iter().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(format!("{}. {}", i + 1, action.display()));
                            if ui.small_button("删除").clicked() {
                                to_remove = Some(i);
                            }
                        });
                    }
                    if let Some(idx) = to_remove {
                        self.action_configs.remove(idx);
                    }
                    if self.action_configs.is_empty() {
                        ui.label("(无配置)");
                    }
                });

            ui.add_space(5.0);

            // 执行按钮
            ui.horizontal(|ui| {
                let is_running = *self.is_running.lock().unwrap();
                let btn_text = if is_running { "执行中..." } else { "执行测试 (3秒后)" };

                if ui.add_enabled(!is_running, egui::Button::new(btn_text)).clicked() {
                    self.run_action_test();
                }

                if ui.button("复制代码").clicked() {
                    self.copy_action_code(ui);
                }

                if ui.button("清空配置").clicked() {
                    self.action_configs.clear();
                    self.action_msg.clear();
                }
            });

            // 检查执行状态更新消息
            if !*self.is_running.lock().unwrap() && self.action_msg == "3秒后开始执行..." {
                self.action_msg = "执行完成".to_string();
            }
        });
    }
}
