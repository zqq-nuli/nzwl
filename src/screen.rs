//! 屏幕截图模块
//!
//! 使用 win-screenshot 进行屏幕区域截图

use anyhow::{anyhow, Context, Result};
use image::{DynamicImage, RgbImage};
use win_screenshot::prelude::*;

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
