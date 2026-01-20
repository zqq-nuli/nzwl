//! OCR 测试工具 - GUI 版本
//!
//! 用于测试指定屏幕区域的 OCR 识别效果

use eframe::egui;

// 导入主项目的模块
use nz_rust::ocr::{init_ocr, ocr_screen, OcrResultItem};
use nz_rust::screen::capture_region;

fn main() -> eframe::Result<()> {
    // 初始化 OCR 引擎
    println!("正在初始化 OCR 引擎...");
    if let Err(e) = init_ocr() {
        eprintln!("OCR 初始化失败: {}", e);
        return Ok(());
    }
    println!("OCR 引擎初始化完成");

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([600.0, 700.0])
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
        match ocr_screen(start_x, start_y, width, height, false, false) {
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

            ui.add_space(10.0);

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
                .max_height(250.0)
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
        });
    }
}
