//! 地图策略编辑器
//!
//! 在游戏截图上可视化编辑陷阱放置位置和波次时序，导出 JSON 策略文件。

use eframe::egui;
use std::path::PathBuf;

use nz_rust::strategy::{Building, MovementPhase, Strategy, StrategyMeta, screen_to_grid};

/// 波次颜色（用于区分不同波次的建筑标记）
const WAVE_COLORS: &[(u8, u8, u8)] = &[
    (66, 133, 244),  // 蓝
    (234, 67, 53),   // 红
    (251, 188, 4),   // 黄
    (52, 168, 83),   // 绿
    (171, 71, 188),  // 紫
    (255, 112, 67),  // 橙
];

fn wave_color(wave: u32) -> egui::Color32 {
    let idx = ((wave.saturating_sub(1)) as usize) % WAVE_COLORS.len();
    let (r, g, b) = WAVE_COLORS[idx];
    egui::Color32::from_rgb(r, g, b)
}

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 800.0])
            .with_title("地图策略编辑器"),
        ..Default::default()
    };

    eframe::run_native(
        "地图策略编辑器",
        options,
        Box::new(|cc| {
            // 加载中文字体
            let mut fonts = egui::FontDefinitions::default();
            if let Ok(font_data) = std::fs::read("C:\\Windows\\Fonts\\msyh.ttc") {
                fonts.font_data.insert(
                    "msyh".to_owned(),
                    egui::FontData::from_owned(font_data).into(),
                );
// PLACEHOLDER_EDITOR_FONTS
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

            Ok(Box::new(MapEditorApp::default()))
        }),
    )
}

/// 编辑器主状态
struct MapEditorApp {
    /// 当前策略
    strategy: Strategy,
    /// 底图纹理
    map_texture: Option<egui::TextureHandle>,
    /// 底图原始尺寸
    map_size: [usize; 2],
    /// 画布缩放
    zoom: f32,
    /// 画布平移偏移
    pan: egui::Vec2,
    /// 显示网格
    show_grid: bool,

    // 右侧面板状态
    /// 新建筑：名称
    new_building_name: String,
    /// 新建筑：快捷键
    new_building_key: String,
    /// 新建筑：波次
    new_building_wave: u32,
    /// 新建筑：是否 late
    new_building_late: bool,
    /// 是否处于放置模式（点击地图放置建筑）
    placing_mode: bool,
    /// 选中的建筑索引
    selected_building: Option<usize>,

    /// 移动阶段 JSON 导入文本
    movement_json: String,
    /// 状态消息
    status_msg: String,
    /// 下一个建筑 ID 计数器
    next_id: u32,
}

impl Default for MapEditorApp {
    fn default() -> Self {
        Self {
            strategy: Strategy {
                meta: StrategyMeta {
                    name: "新策略".to_string(),
                    difficulty: "困难".to_string(),
                    screenshot: String::new(),
                    grid_pixel_size: 64.0,
                    offset_x: 0.0,
                    offset_y: 0.0,
                },
                shop_order: Vec::new(),
                buildings: Vec::new(),
                upgrades: Vec::new(),
                demolishes: Vec::new(),
                movement_phases: Vec::new(),
            },
            map_texture: None,
            map_size: [1920, 1080],
            zoom: 0.6,
            pan: egui::Vec2::ZERO,
            show_grid: false,
            new_building_name: "自修复磁暴塔".to_string(),
            new_building_key: "5".to_string(),
            new_building_wave: 1,
            new_building_late: false,
            placing_mode: false,
            selected_building: None,
            movement_json: String::new(),
            status_msg: String::new(),
            next_id: 1,
        }
    }
}
// PLACEHOLDER_EDITOR_IMPL

impl MapEditorApp {
    /// 加载底图 PNG
    fn load_map_image(&mut self, ctx: &egui::Context, path: &PathBuf) {
        match image::open(path) {
            Ok(img) => {
                let rgba = img.to_rgba8();
                let size = [rgba.width() as usize, rgba.height() as usize];
                let pixels: Vec<egui::Color32> = rgba
                    .pixels()
                    .map(|p| egui::Color32::from_rgba_unmultiplied(p[0], p[1], p[2], p[3]))
                    .collect();
                let color_image = egui::ColorImage { size, pixels };
                self.map_texture = Some(ctx.load_texture(
                    "map_bg",
                    color_image,
                    egui::TextureOptions::LINEAR,
                ));
                self.map_size = size;
                self.strategy.meta.screenshot = path.to_string_lossy().to_string();
                self.status_msg = format!("已加载底图: {}x{}", size[0], size[1]);
            }
            Err(e) => {
                self.status_msg = format!("加载图片失败: {}", e);
            }
        }
    }

    /// 屏幕坐标转画布 widget 坐标
    fn screen_to_canvas(&self, screen_x: i32, screen_y: i32, canvas_origin: egui::Pos2) -> egui::Pos2 {
        egui::pos2(
            canvas_origin.x + screen_x as f32 * self.zoom + self.pan.x,
            canvas_origin.y + screen_y as f32 * self.zoom + self.pan.y,
        )
    }

    /// 画布 widget 坐标转屏幕坐标
    fn canvas_to_screen(&self, canvas_pos: egui::Pos2, canvas_origin: egui::Pos2) -> (i32, i32) {
        let sx = ((canvas_pos.x - canvas_origin.x - self.pan.x) / self.zoom) as i32;
        let sy = ((canvas_pos.y - canvas_origin.y - self.pan.y) / self.zoom) as i32;
        (sx, sy)
    }

    /// 生成新建筑 ID
    fn gen_id(&mut self) -> String {
        let id = format!("b_{}", self.next_id);
        self.next_id += 1;
        id
    }
}
// PLACEHOLDER_EDITOR_APP_IMPL

impl eframe::App for MapEditorApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 右侧面板
        egui::SidePanel::right("right_panel")
            .min_width(280.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    self.draw_right_panel(ui, ctx);
                });
            });

        // 中央画布
        egui::CentralPanel::default().show(ctx, |ui| {
            self.draw_canvas(ui, ctx);
        });
    }
}

impl MapEditorApp {
    fn draw_right_panel(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        // === 文件操作 ===
        ui.heading("文件");
        ui.horizontal(|ui| {
            if ui.button("加载底图").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("图片", &["png", "jpg", "jpeg", "bmp"])
                    .pick_file()
                {
                    self.load_map_image(ctx, &path);
                }
            }
            if ui.button("加载策略").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("JSON", &["json"])
                    .pick_file()
                {
                    match Strategy::load(&path) {
                        Ok(s) => {
                            // 更新 next_id
                            self.next_id = s.buildings.len() as u32 + 1;
                            self.strategy = s;
                            self.status_msg = "策略加载成功".to_string();
                            // 尝试加载截图
                            let screenshot = self.strategy.meta.screenshot.clone();
                            if !screenshot.is_empty() {
                                let p = PathBuf::from(&screenshot);
                                if p.exists() {
                                    self.load_map_image(ctx, &p);
                                }
                            }
                        }
                        Err(e) => self.status_msg = format!("加载失败: {}", e),
                    }
                }
            }
        });
        if ui.button("导出策略 JSON").clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("JSON", &["json"])
                .set_file_name("strategy.json")
                .save_file()
            {
                match self.strategy.save(&path) {
                    Ok(_) => self.status_msg = format!("已导出: {}", path.display()),
                    Err(e) => self.status_msg = format!("导出失败: {}", e),
                }
            }
        }

        ui.separator();

        // === 策略信息 ===
        ui.heading("策略信息");
        ui.horizontal(|ui| {
            ui.label("名称:");
            ui.text_edit_singleline(&mut self.strategy.meta.name);
        });
        ui.horizontal(|ui| {
            ui.label("难度:");
            egui::ComboBox::from_id_salt("difficulty")
                .selected_text(&self.strategy.meta.difficulty)
                .show_ui(ui, |ui| {
                    for d in &["普通", "困难", "炼狱"] {
                        ui.selectable_value(
                            &mut self.strategy.meta.difficulty,
                            d.to_string(),
                            *d,
                        );
                    }
                });
        });

        ui.separator();

        // === 网格设置 ===
        ui.heading("网格设置");
        ui.checkbox(&mut self.show_grid, "显示网格");
        ui.horizontal(|ui| {
            ui.label("格子大小:");
            ui.add(
                egui::DragValue::new(&mut self.strategy.meta.grid_pixel_size)
                    .range(8.0..=256.0)
                    .speed(1.0),
            );
        });
        ui.horizontal(|ui| {
            ui.label("偏移 X:");
            ui.add(egui::DragValue::new(&mut self.strategy.meta.offset_x).speed(1.0));
        });
        ui.horizontal(|ui| {
            ui.label("偏移 Y:");
            ui.add(egui::DragValue::new(&mut self.strategy.meta.offset_y).speed(1.0));
        });

        ui.separator();
// PLACEHOLDER_RIGHT_PANEL_2

        // === 商店购买顺序 ===
        ui.heading("商店购买顺序");
        let mut shop_changed = false;
        let mut shop_remove: Option<usize> = None;
        for (i, name) in self.strategy.shop_order.iter().enumerate() {
            ui.horizontal(|ui| {
                ui.label(format!("{}. {}", i + 1, name));
                if ui.small_button("删除").clicked() {
                    shop_remove = Some(i);
                    shop_changed = true;
                }
            });
        }
        if let Some(idx) = shop_remove {
            self.strategy.shop_order.remove(idx);
        }
        ui.horizontal(|ui| {
            if ui.button("添加").clicked() {
                self.strategy
                    .shop_order
                    .push(self.new_building_name.clone());
                shop_changed = true;
            }
            ui.label("(使用下方建筑名称)");
        });
        if shop_changed {
            self.status_msg = "商店顺序已更新".to_string();
        }

        ui.separator();

        // === 放置建筑 ===
        ui.heading("放置建筑");
        ui.horizontal(|ui| {
            ui.label("名称:");
            ui.text_edit_singleline(&mut self.new_building_name);
        });
        ui.horizontal(|ui| {
            ui.label("快捷键:");
            ui.text_edit_singleline(&mut self.new_building_key);
        });
        ui.horizontal(|ui| {
            ui.label("波次:");
            ui.add(
                egui::DragValue::new(&mut self.new_building_wave)
                    .range(1..=20)
                    .speed(0.1),
            );
            ui.checkbox(&mut self.new_building_late, "波中放置");
        });

        let place_label = if self.placing_mode {
            "取消放置"
        } else {
            "点击地图放置"
        };
        if ui.button(place_label).clicked() {
            self.placing_mode = !self.placing_mode;
            if self.placing_mode {
                self.selected_building = None;
                self.status_msg = "点击地图放置建筑".to_string();
            }
        }

        ui.separator();

        // === 建筑列表 ===
        ui.heading(format!("建筑列表 ({})", self.strategy.buildings.len()));
        let mut remove_idx: Option<usize> = None;
        egui::ScrollArea::vertical()
            .id_salt("building_list")
            .max_height(200.0)
            .show(ui, |ui| {
                for (i, b) in self.strategy.buildings.iter().enumerate() {
                    let color = wave_color(b.wave);
                    let selected = self.selected_building == Some(i);
                    let label = format!(
                        "{} [{}] W{}{} ({},{})",
                        b.name,
                        b.trap_key,
                        b.wave,
                        if b.is_late { "+" } else { "" },
                        b.screen_x,
                        b.screen_y
                    );
                    ui.horizontal(|ui| {
                        ui.colored_label(color, "■");
                        if ui.selectable_label(selected, &label).clicked() {
                            self.selected_building = Some(i);
                            self.placing_mode = false;
                        }
                        if ui.small_button("×").clicked() {
                            remove_idx = Some(i);
                        }
                    });
                }
            });
        if let Some(idx) = remove_idx {
            self.strategy.buildings.remove(idx);
            self.selected_building = None;
            self.status_msg = "已删除建筑".to_string();
        }

        ui.separator();

        // === 移动阶段 ===
        ui.heading(format!(
            "移动阶段 ({})",
            self.strategy.movement_phases.len()
        ));
        for phase in &self.strategy.movement_phases {
            ui.label(format!(
                "  {} [{}] ({} 步)",
                phase.name,
                phase.trigger,
                phase.actions.len()
            ));
        }
        ui.label("导入移动阶段 JSON:");
        ui.add(
            egui::TextEdit::multiline(&mut self.movement_json)
                .desired_rows(4)
                .desired_width(f32::INFINITY),
        );
        if ui.button("导入移动阶段").clicked() {
            match serde_json::from_str::<Vec<MovementPhase>>(&self.movement_json) {
                Ok(phases) => {
                    self.strategy.movement_phases = phases;
                    self.status_msg = format!(
                        "已导入 {} 个移动阶段",
                        self.strategy.movement_phases.len()
                    );
                }
                Err(e) => {
                    self.status_msg = format!("JSON 解析失败: {}", e);
                }
            }
        }

        ui.separator();

        // 状态栏
        if !self.status_msg.is_empty() {
            ui.colored_label(egui::Color32::YELLOW, &self.status_msg);
        }
    }
// PLACEHOLDER_CANVAS

    fn draw_canvas(&mut self, ui: &mut egui::Ui, _ctx: &egui::Context) {
        let (response, painter) = ui.allocate_painter(
            ui.available_size(),
            egui::Sense::click_and_drag(),
        );
        let canvas_rect = response.rect;
        let canvas_origin = canvas_rect.min;

        // 背景
        painter.rect_filled(canvas_rect, 0.0, egui::Color32::from_gray(30));

        // 滚轮缩放
        let scroll_delta = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll_delta != 0.0 && canvas_rect.contains(ui.input(|i| i.pointer.hover_pos().unwrap_or_default())) {
            let old_zoom = self.zoom;
            self.zoom = (self.zoom * (1.0 + scroll_delta * 0.002)).clamp(0.1, 5.0);
            // 以鼠标位置为中心缩放
            if let Some(pointer) = ui.input(|i| i.pointer.hover_pos()) {
                let rel = pointer - canvas_origin - self.pan;
                self.pan += rel * (1.0 - self.zoom / old_zoom);
            }
        }

        // 中键拖拽平移
        if response.dragged_by(egui::PointerButton::Middle) {
            self.pan += response.drag_delta();
        }

        // 绘制底图
        if let Some(texture) = &self.map_texture {
            let img_size = egui::vec2(
                self.map_size[0] as f32 * self.zoom,
                self.map_size[1] as f32 * self.zoom,
            );
            let img_rect = egui::Rect::from_min_size(
                canvas_origin + self.pan,
                img_size,
            );
            painter.image(
                texture.id(),
                img_rect,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
        }

        // 绘制网格
        if self.show_grid {
            let grid = self.strategy.meta.grid_pixel_size;
            let ox = self.strategy.meta.offset_x;
            let oy = self.strategy.meta.offset_y;
            let grid_color = egui::Color32::from_rgba_unmultiplied(255, 255, 255, 30);

            let w = self.map_size[0] as f32;
            let h = self.map_size[1] as f32;

            // 垂直线
            let mut x = ox;
            while x < w {
                let p1 = self.screen_to_canvas(x as i32, 0, canvas_origin);
                let p2 = self.screen_to_canvas(x as i32, h as i32, canvas_origin);
                painter.line_segment([p1, p2], egui::Stroke::new(1.0, grid_color));
                x += grid;
            }
            // 水平线
            let mut y = oy;
            while y < h {
                let p1 = self.screen_to_canvas(0, y as i32, canvas_origin);
                let p2 = self.screen_to_canvas(w as i32, y as i32, canvas_origin);
                painter.line_segment([p1, p2], egui::Stroke::new(1.0, grid_color));
                y += grid;
            }
        }

        // 绘制建筑标记
        let building_size = 20.0 * self.zoom;
        for (i, b) in self.strategy.buildings.iter().enumerate() {
            let center = self.screen_to_canvas(b.screen_x, b.screen_y, canvas_origin);
            let color = wave_color(b.wave);
            let selected = self.selected_building == Some(i);

            // 矩形标记
            let rect = egui::Rect::from_center_size(
                center,
                egui::vec2(building_size, building_size),
            );
            painter.rect_filled(rect, 2.0, color);

            if selected {
                painter.rect_stroke(
                    rect.expand(2.0),
                    2.0,
                    egui::Stroke::new(2.0, egui::Color32::WHITE),
                    egui::StrokeKind::Outside,
                );
            }

            // 波次标签
            let label = format!("W{}", b.wave);
            painter.text(
                center + egui::vec2(0.0, -building_size * 0.7),
                egui::Align2::CENTER_BOTTOM,
                &label,
                egui::FontId::proportional(10.0 * self.zoom.max(0.5)),
                egui::Color32::WHITE,
            );
        }

        // 鼠标悬停信息
        if let Some(pointer) = response.hover_pos() {
            let (sx, sy) = self.canvas_to_screen(pointer, canvas_origin);
            if sx >= 0 && sy >= 0 && sx < self.map_size[0] as i32 && sy < self.map_size[1] as i32 {
                // 坐标提示
                let info = if self.placing_mode {
                    format!("点击放置 ({}, {})", sx, sy)
                } else {
                    format!("({}, {})", sx, sy)
                };
                painter.text(
                    pointer + egui::vec2(15.0, -15.0),
                    egui::Align2::LEFT_BOTTOM,
                    &info,
                    egui::FontId::proportional(12.0),
                    egui::Color32::WHITE,
                );
            }
        }

        // 左键点击处理
        if response.clicked() {
            if let Some(pointer) = response.interact_pointer_pos() {
                let (sx, sy) = self.canvas_to_screen(pointer, canvas_origin);

                if self.placing_mode {
                    // 放置新建筑
                    let (gx, gy) = screen_to_grid(sx, sy, &self.strategy.meta);
                    let id = self.gen_id();
                    self.strategy.buildings.push(Building {
                        id,
                        name: self.new_building_name.clone(),
                        trap_key: self.new_building_key.clone(),
                        screen_x: sx,
                        screen_y: sy,
                        grid_x: gx,
                        grid_y: gy,
                        wave: self.new_building_wave,
                        is_late: self.new_building_late,
                    });
                    self.status_msg = format!("已放置 {} @ ({}, {})", self.new_building_name, sx, sy);
                } else {
                    // 选中建筑
                    let threshold = 15.0;
                    let mut found = None;
                    for (i, b) in self.strategy.buildings.iter().enumerate() {
                        let dx = (b.screen_x - sx) as f32;
                        let dy = (b.screen_y - sy) as f32;
                        if (dx * dx + dy * dy).sqrt() < threshold / self.zoom + 10.0 {
                            found = Some(i);
                            break;
                        }
                    }
                    self.selected_building = found;
                }
            }
        }

        // 右键删除建筑
        if response.secondary_clicked() {
            if let Some(pointer) = response.interact_pointer_pos() {
                let (sx, sy) = self.canvas_to_screen(pointer, canvas_origin);
                let threshold = 15.0;
                let mut found = None;
                for (i, b) in self.strategy.buildings.iter().enumerate() {
                    let dx = (b.screen_x - sx) as f32;
                    let dy = (b.screen_y - sy) as f32;
                    if (dx * dx + dy * dy).sqrt() < threshold / self.zoom + 10.0 {
                        found = Some(i);
                        break;
                    }
                }
                if let Some(idx) = found {
                    let name = self.strategy.buildings[idx].name.clone();
                    self.strategy.buildings.remove(idx);
                    self.selected_building = None;
                    self.status_msg = format!("已删除: {}", name);
                }
            }
        }

        // 缩放/坐标信息栏
        let info = format!("缩放: {:.0}%  |  建筑: {}", self.zoom * 100.0, self.strategy.buildings.len());
        painter.text(
            canvas_rect.left_bottom() + egui::vec2(5.0, -5.0),
            egui::Align2::LEFT_BOTTOM,
            &info,
            egui::FontId::proportional(12.0),
            egui::Color32::from_gray(180),
        );
    }
}
