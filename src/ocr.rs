//! OCR 模块
//!
//! 使用 ocr-rs (MNN 后端) 进行文字识别

use anyhow::{Context, Result};
use image::RgbImage;
use ocr_rs::OcrEngine;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::sync::Mutex;
use std::sync::OnceLock;
use std::time::Instant;
use strsim::jaro_winkler;

/// OCR 引擎单例
static OCR_ENGINE: OnceLock<Mutex<OcrEngine>> = OnceLock::new();

/// 帧差跳过缓存
static FRAME_CACHE: OnceLock<Mutex<FrameCache>> = OnceLock::new();

/// 帧缓存结构
struct FrameCache {
    hash: Option<u64>,
    result: Option<Vec<OcrResultItem>>,
}

impl Default for FrameCache {
    fn default() -> Self {
        Self {
            hash: None,
            result: None,
        }
    }
}

/// OCR 识别结果（自定义结构，方便使用）
#[derive(Debug, Clone)]
pub struct OcrResultItem {
    /// 识别的文字
    pub text: String,
    /// 文字框坐标 [[x1,y1], [x2,y2], [x3,y3], [x4,y4]]
    pub box_points: [[i32; 2]; 4],
    /// 置信度
    pub score: f32,
}

impl OcrResultItem {
    /// 获取文字框中心点
    pub fn center(&self) -> (i32, i32) {
        let x = (self.box_points[0][0] + self.box_points[2][0]) / 2;
        let y = (self.box_points[0][1] + self.box_points[2][1]) / 2;
        (x, y)
    }
}

/// 初始化 OCR 引擎
pub fn init_ocr() -> Result<()> {
    let models_dir = Path::new("models");

    // MNN 格式模型文件 (PP-OCRv4)
    let det_model = models_dir.join("ch_PP-OCRv4_det_infer.mnn");
    let rec_model = models_dir.join("ch_PP-OCRv4_rec_infer.mnn");
    let keys_file = models_dir.join("ppocr_keys_v4.txt");

    // 检查模型文件是否存在
    if !det_model.exists() {
        anyhow::bail!(
            "检测模型不存在: {}\n请下载 MNN 格式的 PaddleOCR 模型文件到 models/ 目录",
            det_model.display()
        );
    }

    if !rec_model.exists() {
        anyhow::bail!(
            "识别模型不存在: {}\n请下载 MNN 格式的 PaddleOCR 模型文件到 models/ 目录",
            rec_model.display()
        );
    }

    if !keys_file.exists() {
        anyhow::bail!(
            "字符集文件不存在: {}\n请下载 ppocr_keys_v1.txt 到 models/ 目录",
            keys_file.display()
        );
    }

    // 初始化 OCR 引擎
    let engine = OcrEngine::new(
        det_model.to_str().unwrap(),
        rec_model.to_str().unwrap(),
        keys_file.to_str().unwrap(),
        None, // 使用默认配置
    )
    .map_err(|e| anyhow::anyhow!("初始化 OCR 引擎失败: {:?}", e))?;

    OCR_ENGINE
        .set(Mutex::new(engine))
        .map_err(|_| anyhow::anyhow!("OCR 引擎已初始化"))?;

    // 初始化帧缓存
    let _ = FRAME_CACHE.set(Mutex::new(FrameCache::default()));

    Ok(())
}

/// 计算图像哈希值（用于帧差检测）
fn compute_image_hash(img: &RgbImage) -> u64 {
    // 缩小到 32x32 再计算哈希
    let small = image::imageops::resize(img, 32, 32, image::imageops::FilterType::Nearest);
    let mut hasher = DefaultHasher::new();
    small.as_raw().hash(&mut hasher);
    hasher.finish()
}

/// 检查是否应该跳过当前帧（帧未变化）
fn should_skip_frame(img: &RgbImage) -> bool {
    let current_hash = compute_image_hash(img);

    if let Some(cache) = FRAME_CACHE.get() {
        if let Ok(cache) = cache.lock() {
            if let Some(prev_hash) = cache.hash {
                return prev_hash == current_hash && cache.result.is_some();
            }
        }
    }
    false
}

/// 获取缓存的 OCR 结果
fn get_cached_result() -> Option<Vec<OcrResultItem>> {
    FRAME_CACHE.get()?.lock().ok()?.result.clone()
}

/// 更新帧缓存
fn update_frame_cache(img: &RgbImage, result: &[OcrResultItem]) {
    if let Some(cache) = FRAME_CACHE.get() {
        if let Ok(mut cache) = cache.lock() {
            cache.hash = Some(compute_image_hash(img));
            cache.result = Some(result.to_vec());
        }
    }
}

/// 清空帧差缓存（用于场景切换时）
pub fn clear_frame_cache() {
    if let Some(cache) = FRAME_CACHE.get() {
        if let Ok(mut cache) = cache.lock() {
            cache.hash = None;
            cache.result = None;
        }
    }
}

/// 对图像进行 OCR 识别
///
/// # Arguments
/// * `img` - RGB 图像
/// * `use_frame_skip` - 是否启用帧差跳过
/// * `debug` - 是否输出调试信息
///
/// # Returns
/// 识别结果列表
pub fn ocr_image(img: &RgbImage, use_frame_skip: bool, debug: bool) -> Result<Vec<OcrResultItem>> {
    let start = Instant::now();

    // 帧差跳过检测
    if use_frame_skip && should_skip_frame(img) {
        if debug {
            println!("OCR: 帧未变化，复用缓存结果");
        }
        return Ok(get_cached_result().unwrap_or_default());
    }

    // 获取 OCR 引擎
    let engine = OCR_ENGINE
        .get()
        .context("OCR 引擎未初始化")?
        .lock()
        .map_err(|e| anyhow::anyhow!("获取 OCR 引擎锁失败: {}", e))?;

    // 转换图像格式为 DynamicImage
    let dynamic_img = image::DynamicImage::ImageRgb8(img.clone());

    // 执行 OCR
    let ocr_start = Instant::now();
    let raw_results = engine
        .recognize(&dynamic_img)
        .map_err(|e| anyhow::anyhow!("OCR 识别失败: {:?}", e))?;
    let ocr_time = ocr_start.elapsed();

    // 转换结果格式
    let results: Vec<OcrResultItem> = raw_results
        .into_iter()
        .map(|block| {
            // 获取边界框坐标 - 使用 rect 字段
            let rect = &block.bbox.rect;
            let x = rect.left() as i32;
            let y = rect.top() as i32;
            let w = rect.width() as i32;
            let h = rect.height() as i32;
            OcrResultItem {
                text: block.text.clone(),
                box_points: [[x, y], [x + w, y], [x + w, y + h], [x, y + h]],
                score: block.bbox.score,
            }
        })
        .collect();

    // 更新缓存
    if use_frame_skip {
        update_frame_cache(img, &results);
    }

    let total_time = start.elapsed();

    if debug {
        let texts: Vec<&str> = results.iter().map(|r| r.text.as_str()).collect();
        println!(
            "OCR: {} texts | 总耗时: {:?} | 识别: {:?} | texts={:?}",
            results.len(),
            total_time,
            ocr_time,
            texts
        );
    }

    Ok(results)
}

/// 截取屏幕区域并进行 OCR
///
/// # Arguments
/// * `x`, `y`, `width`, `height` - 屏幕区域
/// * `use_frame_skip` - 是否启用帧差跳过
/// * `debug` - 是否输出调试信息
pub fn ocr_screen(
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    use_frame_skip: bool,
    debug: bool,
) -> Result<Vec<OcrResultItem>> {
    // 截取屏幕区域
    let img = crate::screen::capture_region(x, y, width, height)?;

    // 执行 OCR
    let mut results = ocr_image(&img, use_frame_skip, debug)?;

    // 调整坐标为屏幕绝对坐标
    for result in &mut results {
        for point in &mut result.box_points {
            point[0] += x;
            point[1] += y;
        }
    }

    Ok(results)
}

/// 在 OCR 结果中查找指定文字
///
/// # Arguments
/// * `results` - OCR 结果列表
/// * `target_text` - 目标文字
/// * `similarity_threshold` - 相似度阈值 (0.0-1.0)
///
/// # Returns
/// 找到的结果（如果有）
pub fn find_text<'a>(
    results: &'a [OcrResultItem],
    target_text: &str,
    similarity_threshold: f64,
) -> Option<&'a OcrResultItem> {
    results.iter().find(|r| {
        let similarity = jaro_winkler(&r.text, target_text);
        similarity >= similarity_threshold
    })
}

/// 在 OCR 结果中查找包含指定文字的结果
pub fn find_text_contains<'a>(
    results: &'a [OcrResultItem],
    target_text: &str,
) -> Option<&'a OcrResultItem> {
    results.iter().find(|r| r.text.contains(target_text))
}

// ============== 测试模块 ==============
#[cfg(test)]
mod tests {
    use super::*;

    /// 测试 OCR 引擎初始化
    #[test]
    fn test_init_ocr() {
        let result = init_ocr();
        assert!(result.is_ok(), "OCR 引擎初始化失败: {:?}", result.err());
    }

    /// 测试指定屏幕区域的 OCR
    /// 运行前请确保屏幕上有可识别的文字
    #[test]
    fn test_ocr_screen_region() {
        // 先初始化 OCR
        init_ocr().expect("OCR 初始化失败");

        // 测试屏幕左上角区域 (0, 0) 到 (400, 300)
        let results = ocr_screen(0, 0, 400, 300, false, true).expect("OCR 失败");

        println!("识别到 {} 个文字区域:", results.len());
        for r in &results {
            println!(
                "  文字: '{}' | 位置: {:?} | 置信度: {:.2}",
                r.text,
                r.center(),
                r.score
            );
        }

        // 这里只验证 OCR 能运行，不验证具体结果
        // 因为屏幕内容会变化
    }

    /// 测试全屏 OCR
    #[test]
    fn test_ocr_fullscreen() {
        init_ocr().expect("OCR 初始化失败");

        let results = ocr_screen(0, 0, 1920, 1080, false, true).expect("OCR 失败");

        println!("全屏识别到 {} 个文字区域", results.len());
        for r in &results {
            println!("  '{}'", r.text);
        }
    }

    /// 测试自定义区域 - 你可以修改这个测试的坐标
    #[test]
    fn test_ocr_custom_region() {
        init_ocr().expect("OCR 初始化失败");

        // === 修改这里的坐标来测试你想要的区域 ===
        let x = 201;
        let y = 132;
        let width = 67;
        let height = 20;
        // =========================================

        println!("测试区域: ({}, {}) - {}x{}", x, y, width, height);

        let results = ocr_screen(x, y, width, height, false, true).expect("OCR 失败");

        println!("识别结果:");
        if results.is_empty() {
            println!("  (无文字)");
        } else {
            for r in &results {
                println!("  文字: '{}' | 中心点: {:?}", r.text, r.center());
            }
        }
    }

    /// 测试查找特定文字
    #[test]
    fn test_find_specific_text() {
        init_ocr().expect("OCR 初始化失败");

        let results = ocr_screen(0, 0, 1920, 1080, false, false).expect("OCR 失败");

        // 查找包含 "开始" 的文字
        if let Some(item) = find_text_contains(&results, "开始") {
            println!("找到 '开始': 位置 {:?}", item.center());
        } else {
            println!("未找到 '开始'");
        }
    }

    /// 测试从图像文件进行 OCR
    #[test]
    fn test_ocr_from_image_file() {
        init_ocr().expect("OCR 初始化失败");

        // 如果你有测试图片，可以这样测试
        let img_path = "test_image.png";
        if std::path::Path::new(img_path).exists() {
            let img = image::open(img_path).expect("无法打开图片").to_rgb8();

            let results = ocr_image(&img, false, true).expect("OCR 失败");

            println!("图片 OCR 结果:");
            for r in &results {
                println!("  '{}'", r.text);
            }
        } else {
            println!("测试图片 {} 不存在，跳过", img_path);
        }
    }
}
