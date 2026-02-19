//! 鼠标移动测试工具

use nz_rust::logitech;
use std::ffi::c_void;
use std::thread;
use std::time::Duration;

use windows::Win32::Foundation::POINT;
use windows::Win32::UI::WindowsAndMessaging::{GetCursorPos, SystemParametersInfoW, SPI_GETMOUSESPEED};

fn get_cursor_pos() -> (i32, i32) {
    let mut pos = POINT { x: 0, y: 0 };
    unsafe { let _ = GetCursorPos(&mut pos); }
    (pos.x, pos.y)
}

fn get_mouse_speed() -> i32 {
    let mut speed: i32 = 10;
    unsafe {
        let _ = SystemParametersInfoW(
            SPI_GETMOUSESPEED,
            0,
            Some(&mut speed as *mut i32 as *mut c_void),
            Default::default(),
        );
    }
    speed
}

fn main() {
    println!("=== 鼠标移动测试 (Logitech 驱动 + 速度补偿) ===\n");

    // 显示 Windows 鼠标速度设置
    let mouse_speed = get_mouse_speed();
    println!("Windows 鼠标速度设置: {}/20", mouse_speed);

    // 1. 初始化驱动
    println!("\n1. 初始化 Logitech 驱动...");
    match logitech::init() {
        Ok(_) => println!("   [OK] 驱动初始化成功"),
        Err(e) => {
            println!("   [FAIL] 驱动初始化失败: {}", e);
            return;
        }
    }

    // 2. 等待用户准备
    println!("\n2. 3秒后开始测试...");
    for i in (1..=3).rev() {
        println!("   {}...", i);
        thread::sleep(Duration::from_secs(1));
    }

    // 测试: 绝对定位到 (960, 540) - 屏幕中心
    println!("\n=== 测试1: 绝对定位到 (960, 540) ===");
    let (x1, y1) = get_cursor_pos();
    println!("   起始位置: ({}, {})", x1, y1);

    let start = std::time::Instant::now();
    let _ = logitech::mouse_move_absolute(960, 540);
    let elapsed = start.elapsed();

    let (x2, y2) = get_cursor_pos();
    println!("   结束位置: ({}, {})", x2, y2);
    println!("   目标位置: (960, 540)");
    println!("   误差: ({}, {})", x2 - 960, y2 - 540);
    println!("   耗时: {:?}", elapsed);

    thread::sleep(Duration::from_millis(500));

    // 测试: 绝对定位到 (100, 100)
    println!("\n=== 测试2: 绝对定位到 (100, 100) ===");
    let (x1, y1) = get_cursor_pos();
    println!("   起始位置: ({}, {})", x1, y1);

    let start = std::time::Instant::now();
    let _ = logitech::mouse_move_absolute(100, 100);
    let elapsed = start.elapsed();

    let (x2, y2) = get_cursor_pos();
    println!("   结束位置: ({}, {})", x2, y2);
    println!("   目标位置: (100, 100)");
    println!("   误差: ({}, {})", x2 - 100, y2 - 100);
    println!("   耗时: {:?}", elapsed);

    thread::sleep(Duration::from_millis(500));

    // 测试: 绝对定位到 (1820, 980)
    println!("\n=== 测试3: 绝对定位到 (1820, 980) ===");
    let (x1, y1) = get_cursor_pos();
    println!("   起始位置: ({}, {})", x1, y1);

    let start = std::time::Instant::now();
    let _ = logitech::mouse_move_absolute(1820, 980);
    let elapsed = start.elapsed();

    let (x2, y2) = get_cursor_pos();
    println!("   结束位置: ({}, {})", x2, y2);
    println!("   目标位置: (1820, 980)");
    println!("   误差: ({}, {})", x2 - 1820, y2 - 980);
    println!("   耗时: {:?}", elapsed);

    // 测试: Logitech 点击
    println!("\n=== 测试4: Logitech 驱动点击 ===");
    match logitech::left_click() {
        Ok(_) => println!("   [OK] 点击成功"),
        Err(e) => println!("   [FAIL] 点击失败: {}", e),
    }

    println!("\n=== 测试完成 ===");
    println!("\n当前方案:");
    println!("  - 绝对定位: Logitech 驱动 + 迭代修正");
    println!("  - 点击/键盘: Logitech 驱动");
    println!("  - 所有输入都通过驱动层!");
}
