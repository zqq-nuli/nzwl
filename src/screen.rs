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
