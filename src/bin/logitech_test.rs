//! Logitech 驱动测试工具
//!
//! 测试 IbInputSimulator DLL 是否正常工作

use nz_rust::logitech;
use std::thread;
use std::time::Duration;

fn main() {
    println!("=== Logitech 驱动测试 ===\n");

    // 1. 初始化驱动
    println!("1. 初始化驱动...");
    match logitech::init() {
        Ok(_) => println!("   [OK] 驱动初始化成功\n"),
        Err(e) => {
            println!("   [FAIL] 驱动初始化失败: {}", e);
            println!("\n请确保:");
            println!("  - IbInputSimulator.dll 在程序目录下");
            println!("  - Logitech Gaming Software v9.02.65 已安装");
            return;
        }
    }

    println!("2. 等待 3 秒，请打开记事本或其他窗口...");
    for i in (1..=3).rev() {
        println!("   {}...", i);
        thread::sleep(Duration::from_secs(1));
    }
    println!();

    // 2. 测试键盘
    println!("3. 测试键盘输入 (输入 'hello')...");
    let keys = [0x48u16, 0x45, 0x4C, 0x4C, 0x4F]; // H E L L O
    for vk in keys {
        if let Err(e) = logitech::tap_key(vk) {
            println!("   [FAIL] 键盘输入失败: {}", e);
        }
        thread::sleep(Duration::from_millis(100));
    }
    println!("   [OK] 键盘测试完成\n");

    thread::sleep(Duration::from_millis(500));

    // 3. 测试鼠标移动
    println!("4. 测试鼠标相对移动...");
    for _ in 0..4 {
        if let Err(e) = logitech::mouse_move_relative(50, 0) {
            println!("   [FAIL] 鼠标移动失败: {}", e);
        }
        thread::sleep(Duration::from_millis(100));
    }
    for _ in 0..4 {
        if let Err(e) = logitech::mouse_move_relative(-50, 0) {
            println!("   [FAIL] 鼠标移动失败: {}", e);
        }
        thread::sleep(Duration::from_millis(100));
    }
    println!("   [OK] 鼠标移动测试完成\n");

    // 4. 测试鼠标点击（注释掉，避免误操作）
    // println!("5. 测试鼠标左键点击...");
    // if let Err(e) = logitech::left_click() {
    //     println!("   [FAIL] 鼠标点击失败: {}", e);
    // }
    // println!("   [OK] 鼠标点击测试完成\n");

    // 5. 测试鼠标滚轮
    println!("5. 测试鼠标滚轮...");
    for _ in 0..3 {
        if let Err(e) = logitech::mouse_wheel(120) {
            println!("   [FAIL] 滚轮失败: {}", e);
        }
        thread::sleep(Duration::from_millis(200));
    }
    thread::sleep(Duration::from_millis(500));
    for _ in 0..3 {
        if let Err(e) = logitech::mouse_wheel(-120) {
            println!("   [FAIL] 滚轮失败: {}", e);
        }
        thread::sleep(Duration::from_millis(200));
    }
    println!("   [OK] 滚轮测试完成\n");

    // 清理
    logitech::destroy();
    println!("=== 测试完成 ===");
}
