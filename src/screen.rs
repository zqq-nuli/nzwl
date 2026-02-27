//! 屏幕截图模块
//!
//! 使用 win-screenshot 进行屏幕区域截图

use std::sync::OnceLock;

use anyhow::{anyhow, Context, Result};
use image::{DynamicImage, RgbImage};
use win_screenshot::prelude::*;

// ===== 分辨率与坐标缩放 =====

/// 基准分辨率（所有坐标以此为基准定义）
pub const BASE_WIDTH: u32 = 1920;
pub const BASE_HEIGHT: u32 = 1080;

static SCREEN_RESOLUTION: OnceLock<(u32, u32)> = OnceLock::new();

/// 获取屏幕物理分辨率（首次调用检测，后续返回缓存值）
pub fn get_screen_resolution() -> (u32, u32) {
    *SCREEN_RESOLUTION.get_or_init(detect_resolution)
}

fn detect_resolution() -> (u32, u32) {
    // 使用 EnumDisplaySettingsW 获取主显示器物理分辨率
    // 注意：capture_display() 抓取的是整个虚拟桌面（多显示器合并），不适合做分辨率检测
    use windows::Win32::Graphics::Gdi::{EnumDisplaySettingsW, DEVMODEW, ENUM_CURRENT_SETTINGS};
    use windows::core::PCWSTR;

    unsafe {
        let mut devmode: DEVMODEW = std::mem::zeroed();
        devmode.dmSize = std::mem::size_of::<DEVMODEW>() as u16;

        if EnumDisplaySettingsW(PCWSTR::null(), ENUM_CURRENT_SETTINGS, &mut devmode).as_bool() {
            let w = devmode.dmPelsWidth;
            let h = devmode.dmPelsHeight;
            println!("[Screen] 主显示器分辨率: {}x{}", w, h);
            (w, h)
        } else {
            println!("[Screen] 无法检测分辨率，使用默认 1920x1080");
            (BASE_WIDTH, BASE_HEIGHT)
        }
    }
}

/// 获取缩放因子 (scale_x, scale_y)
pub fn get_scale_factors() -> (f64, f64) {
    let (w, h) = get_screen_resolution();
    (w as f64 / BASE_WIDTH as f64, h as f64 / BASE_HEIGHT as f64)
}

/// 缩放 X 坐标（基准 1920 → 实际分辨率）
pub fn scale_x(base: i32) -> i32 {
    let (sx, _) = get_scale_factors();
    (base as f64 * sx).round() as i32
}

/// 缩放 Y 坐标（基准 1080 → 实际分辨率）
pub fn scale_y(base: i32) -> i32 {
    let (_, sy) = get_scale_factors();
    (base as f64 * sy).round() as i32
}

/// 缩放区域 (x, y, w, h) — x/w 按 X 轴，y/h 按 Y 轴
pub fn scale_region(x: i32, y: i32, w: i32, h: i32) -> (i32, i32, i32, i32) {
    (scale_x(x), scale_y(y), scale_x(w), scale_y(h))
}

// ===== 开发分辨率坐标缩放（4K → 实际） =====

/// 开发环境分辨率（策略文件中的坐标以此为基准）
pub const DEV_WIDTH: u32 = 3840;
pub const DEV_HEIGHT: u32 = 2160;

/// 缩放 X 坐标（开发4K → 实际分辨率）
pub fn dev_x(x: i32) -> i32 {
    let (w, _) = get_screen_resolution();
    (x as f64 * w as f64 / DEV_WIDTH as f64).round() as i32
}

/// 缩放 Y 坐标（开发4K → 实际分辨率）
pub fn dev_y(y: i32) -> i32 {
    let (_, h) = get_screen_resolution();
    (y as f64 * h as f64 / DEV_HEIGHT as f64).round() as i32
}

/// 全屏区域（实际分辨率）
pub fn full_screen_region() -> (i32, i32, i32, i32) {
    let (w, h) = get_screen_resolution();
    (0, 0, w as i32, h as i32)
}

/// 截取屏幕指定区域
///
/// # Arguments
/// * `x` - 左上角 X 坐标
/// * `y` - 左上角 Y 坐标
/// * `width` - 区域宽度
/// * `height` - 区域高度
///
/// # Returns
/// RGB 格式的图像
pub fn capture_region(x: i32, y: i32, width: i32, height: i32) -> Result<RgbImage> {
    // 使用 win-screenshot 截取屏幕
    let buf = capture_display()
        .map_err(|e| anyhow!("截取屏幕失败: {:?}", e))?;

    // 转换为 image crate 的格式
    let img = DynamicImage::ImageRgba8(
        image::RgbaImage::from_raw(buf.width, buf.height, buf.pixels)
            .context("无法创建图像缓冲区")?,
    );

    // 裁剪到指定区域
    let cropped = img.crop_imm(x as u32, y as u32, width as u32, height as u32);

    Ok(cropped.to_rgb8())
}

/// 截取全屏
pub fn capture_fullscreen() -> Result<RgbImage> {
    let buf = capture_display()
        .map_err(|e| anyhow!("截取屏幕失败: {:?}", e))?;

    let img = DynamicImage::ImageRgba8(
        image::RgbaImage::from_raw(buf.width, buf.height, buf.pixels)
            .context("无法创建图像缓冲区")?,
    );

    Ok(img.to_rgb8())
}

/// 保存截图到文件（用于调试）
pub fn save_screenshot(img: &RgbImage, path: &str) -> Result<()> {
    img.save(path).context("保存截图失败")?;
    println!("截图已保存: {}", path);
    Ok(())
}

/// 获取屏幕某个坐标点的颜色
///
/// # Returns
/// 返回 RGB 颜色值 (0xRRGGBB 格式)
pub fn get_pixel_color(x: i32, y: i32) -> Result<u32> {
    let buf = capture_display()
        .map_err(|e| anyhow!("截取屏幕失败: {:?}", e))?;

    let img = image::RgbaImage::from_raw(buf.width, buf.height, buf.pixels)
        .context("无法创建图像缓冲区")?;

    let pixel = img.get_pixel(x as u32, y as u32);
    let r = pixel[0] as u32;
    let g = pixel[1] as u32;
    let b = pixel[2] as u32;

    Ok((r << 16) | (g << 8) | b)
}

/// 检查屏幕某个坐标点的颜色是否等于指定值
///
/// # Arguments
/// * `x` - X 坐标
/// * `y` - Y 坐标
/// * `expected_color` - 期望的颜色值 (0xRRGGBB 格式)
///
/// # Returns
/// 颜色匹配返回 true，否则返回 false
pub fn check_pixel_color(x: i32, y: i32, expected_color: u32) -> Result<bool> {
    let actual_color = get_pixel_color(x, y)?;
    Ok(actual_color == expected_color)
}

/// 检查屏幕某个坐标点的颜色是否等于指定值（带容差）
///
/// # Arguments
/// * `x` - X 坐标
/// * `y` - Y 坐标
/// * `expected_color` - 期望的颜色值 (0xRRGGBB 格式)
/// * `tolerance` - 每个通道允许的误差值 (0-255)
///
/// # Returns
/// 颜色在容差范围内返回 true，否则返回 false
pub fn check_pixel_color_tolerance(x: i32, y: i32, expected_color: u32, tolerance: u8) -> Result<bool> {
    let actual_color = get_pixel_color(x, y)?;

    let ar = ((actual_color >> 16) & 0xFF) as i32;
    let ag = ((actual_color >> 8) & 0xFF) as i32;
    let ab = (actual_color & 0xFF) as i32;

    let er = ((expected_color >> 16) & 0xFF) as i32;
    let eg = ((expected_color >> 8) & 0xFF) as i32;
    let eb = (expected_color & 0xFF) as i32;

    let t = tolerance as i32;

    Ok((ar - er).abs() <= t && (ag - eg).abs() <= t && (ab - eb).abs() <= t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capture_region() {
        let img = capture_region(0, 0, 100, 100).unwrap();
        assert_eq!(img.width(), 100);
        assert_eq!(img.height(), 100);
    }
}
