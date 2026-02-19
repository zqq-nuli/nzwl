//! 统一输入抽象层
//!
//! 提供统一的键盘鼠标接口，可在不同后端之间切换：
//! - SendInput: Windows 原生 API（默认）
//! - Logitech: 罗技驱动层输入（需要 LGS v9.02.65）
//!
//! # 使用方法
//!
//! ```rust
//! use nz_rust::input::{self, InputBackend};
//!
//! // 初始化（选择后端）
//! input::init(InputBackend::Logitech)?;
//!
//! // 使用统一 API
//! input::left_click();
//! input::tap_key(0x41); // A
//! input::move_to(100, 200);
//! ```

use std::sync::atomic::{AtomicU8, Ordering};
use std::thread;
use std::time::Duration;

use crate::keys;
use crate::logitech;

// ===== 后端类型 =====

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputBackend {
    /// Windows SendInput API（默认）
    SendInput = 0,
    /// Logitech 驱动层输入
    Logitech = 1,
}

impl Default for InputBackend {
    fn default() -> Self {
        Self::SendInput
    }
}

// 当前使用的后端
static CURRENT_BACKEND: AtomicU8 = AtomicU8::new(0);

// ===== 初始化 =====

/// 初始化输入系统
///
/// - `SendInput`: 无需特殊初始化
/// - `Logitech`: 需要加载 DLL 并初始化驱动
pub fn init(backend: InputBackend) -> Result<(), String> {
    match backend {
        InputBackend::SendInput => {
            CURRENT_BACKEND.store(InputBackend::SendInput as u8, Ordering::SeqCst);
            println!("[Input] 使用 SendInput 后端");
            Ok(())
        }
        InputBackend::Logitech => {
            logitech::init()?;
            CURRENT_BACKEND.store(InputBackend::Logitech as u8, Ordering::SeqCst);
            println!("[Input] 使用 Logitech 驱动后端");
            Ok(())
        }
    }
}

/// 获取当前后端
pub fn current_backend() -> InputBackend {
    match CURRENT_BACKEND.load(Ordering::SeqCst) {
        1 => InputBackend::Logitech,
        _ => InputBackend::SendInput,
    }
}

/// 清理资源
pub fn destroy() {
    if current_backend() == InputBackend::Logitech {
        logitech::destroy();
    }
}

// ===== 鼠标操作 =====

/// 相对移动鼠标
pub fn send_relative(dx: i32, dy: i32) {
    match current_backend() {
        InputBackend::SendInput => keys::send_relative(dx, dy),
        InputBackend::Logitech => {
            let _ = logitech::mouse_move_relative(dx, dy);
        }
    }
}

/// 移动鼠标到绝对坐标
pub fn move_to(x: i32, y: i32) {
    match current_backend() {
        InputBackend::SendInput => keys::move_to(x, y),
        InputBackend::Logitech => {
            let _ = logitech::mouse_move_absolute(x, y);
        }
    }
}

/// 鼠标左键点击
pub fn left_click() {
    match current_backend() {
        InputBackend::SendInput => keys::left_click_legacy(),
        InputBackend::Logitech => {
            let _ = logitech::left_click();
        }
    }
}

/// 移动并点击
/// 增加足够的延迟让游戏引擎注册新位置
pub fn click_at(x: i32, y: i32) {
    move_to(x, y);
    // 等待游戏引擎更新鼠标位置（UE4 通常需要 1-2 帧）
    thread::sleep(Duration::from_millis(100));
    left_click();
}

/// 鼠标右键点击
pub fn right_click() {
    match current_backend() {
        InputBackend::SendInput => {
            // keys.rs 没有 right_click，使用 mouse_event
            use windows::Win32::UI::Input::KeyboardAndMouse::{mouse_event, MOUSE_EVENT_FLAGS};
            unsafe {
                mouse_event(MOUSE_EVENT_FLAGS(0x0008), 0, 0, 0, 0); // RIGHTDOWN
                thread::sleep(Duration::from_millis(10));
                mouse_event(MOUSE_EVENT_FLAGS(0x0010), 0, 0, 0, 0); // RIGHTUP
            }
        }
        InputBackend::Logitech => {
            let _ = logitech::right_click();
        }
    }
}

/// 滚动方向
pub use keys::ScrollDirection;

/// 鼠标滚轮滚动
pub fn mouse_scroll(direction: ScrollDirection, count: u32, interval_secs: f64) {
    match current_backend() {
        InputBackend::SendInput => keys::mouse_scroll(direction, count, interval_secs),
        InputBackend::Logitech => {
            let delta: i32 = match direction {
                ScrollDirection::Up => 120,
                ScrollDirection::Down => -120,
            };
            for i in 0..count {
                let _ = logitech::mouse_wheel(delta);
                if i < count - 1 {
                    thread::sleep(Duration::from_secs_f64(interval_secs));
                }
            }
        }
    }
}

// ===== 方向移动（视角转动）=====

/// 视角向左转
pub fn move_left(value: i32) {
    send_relative(-value, 0);
    println!("向左 {}", value);
}

/// 视角向右转
pub fn move_right(value: i32) {
    send_relative(value, 0);
    println!("向右 {}", value);
}

/// 视角向上
pub fn move_up(value: i32) {
    send_relative(0, -value);
    println!("向上 {}", value);
}

/// 视角向下
pub fn move_down(value: i32) {
    send_relative(0, value);
    println!("向下 {}", value);
}

// ===== 键盘操作 =====

/// 按下键
pub fn key_down(vk: u16) {
    match current_backend() {
        InputBackend::SendInput => keys::key_down(vk),
        InputBackend::Logitech => {
            let _ = logitech::key_down(vk);
        }
    }
}

/// 抬起键
pub fn key_up(vk: u16) {
    match current_backend() {
        InputBackend::SendInput => keys::key_up(vk),
        InputBackend::Logitech => {
            let _ = logitech::key_up(vk);
        }
    }
}

/// 点击键（按下并抬起）
pub fn tap_key(vk: u16) {
    key_down(vk);
    thread::sleep(Duration::from_millis(50));
    key_up(vk);
    println!("点击键 0x{:02X}", vk);
}

/// 按住键一段时间
pub fn press_key(vk: u16, duration_secs: f64) {
    key_down(vk);
    println!("按下键 0x{:02X}，持续 {} 秒...", vk, duration_secs);
    thread::sleep(Duration::from_secs_f64(duration_secs));
    key_up(vk);
    println!("松开键 0x{:02X}", vk);
}

/// 按键序列动作类型
pub use keys::KeyAction;

/// 执行按键序列
pub fn press_key_sequence(actions: &[KeyAction]) {
    let mut held_keys: Vec<u16> = Vec::new();

    for (i, action) in actions.iter().enumerate() {
        match action {
            KeyAction::Hold(vk, duration) => {
                if *duration == 0.0 {
                    key_down(*vk);
                    held_keys.push(*vk);
                    println!("[{}] 按住 0x{:02X}", i + 1, vk);
                } else {
                    key_down(*vk);
                    println!("[{}] 按住 0x{:02X} {} 秒...", i + 1, vk, duration);
                    thread::sleep(Duration::from_secs_f64(*duration));
                    key_up(*vk);
                    println!("[{}] 松开 0x{:02X}", i + 1, vk);
                }
            }
            KeyAction::Tap(vk, count) => {
                let count = (*count).max(1);
                for j in 0..count {
                    tap_key(*vk);
                    if j < count - 1 {
                        thread::sleep(Duration::from_millis(100));
                    }
                }
            }
            KeyAction::Release(vk) => {
                key_up(*vk);
                held_keys.retain(|k| k != vk);
                println!("[{}] 松开 0x{:02X}", i + 1, vk);
            }
        }
    }

    // 确保所有按住的键都被松开
    for vk in held_keys {
        key_up(vk);
        println!("清理：松开 0x{:02X}", vk);
    }
}

// ===== 重导出常用虚拟键码 =====

pub use keys::{
    VK_SPACE, VK_RETURN, VK_ESCAPE, VK_TAB, VK_SHIFT, VK_CONTROL, VK_ALT,
    VK_A, VK_B, VK_C, VK_D, VK_E, VK_F, VK_G, VK_H, VK_I, VK_J, VK_K, VK_L, VK_M,
    VK_N, VK_O, VK_P, VK_Q, VK_R, VK_S, VK_T, VK_U, VK_V, VK_W, VK_X, VK_Y, VK_Z,
    VK_0, VK_1, VK_2, VK_3, VK_4, VK_5, VK_6, VK_7, VK_8, VK_9,
    VK_F1, VK_F2,
    get_vk_code,
};
