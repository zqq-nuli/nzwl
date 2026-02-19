//! 键盘鼠标输入模块
//!
//! 使用 Windows SendInput API 和 mouse_event API 实现低级输入
//! 注意：某些游戏会屏蔽 SendInput，需要使用 mouse_event (legacy) 方式

use std::sync::OnceLock;
use std::thread;
use std::time::Duration;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, INPUT_MOUSE, KEYBDINPUT, KEYEVENTF_KEYUP,
    MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP, MOUSEEVENTF_MOVE, MOUSEINPUT, VIRTUAL_KEY,
    mouse_event, MOUSE_EVENT_FLAGS,
};
use windows::Win32::UI::WindowsAndMessaging::SystemParametersInfoA;

// ===== 虚拟键码 =====
pub const VK_SPACE: u16 = 0x20;
pub const VK_RETURN: u16 = 0x0D;
pub const VK_ESCAPE: u16 = 0x1B;
pub const VK_TAB: u16 = 0x09;
pub const VK_SHIFT: u16 = 0x10;
pub const VK_CONTROL: u16 = 0x11;
pub const VK_ALT: u16 = 0x12;

// 字母键 A-Z
pub const VK_A: u16 = 0x41;
pub const VK_B: u16 = 0x42;
pub const VK_C: u16 = 0x43;
pub const VK_D: u16 = 0x44;
pub const VK_E: u16 = 0x45;
pub const VK_F: u16 = 0x46;
pub const VK_G: u16 = 0x47;
pub const VK_H: u16 = 0x48;
pub const VK_I: u16 = 0x49;
pub const VK_J: u16 = 0x4A;
pub const VK_K: u16 = 0x4B;
pub const VK_L: u16 = 0x4C;
pub const VK_M: u16 = 0x4D;
pub const VK_N: u16 = 0x4E;
pub const VK_O: u16 = 0x4F;
pub const VK_P: u16 = 0x50;
pub const VK_Q: u16 = 0x51;
pub const VK_R: u16 = 0x52;
pub const VK_S: u16 = 0x53;
pub const VK_T: u16 = 0x54;
pub const VK_U: u16 = 0x55;
pub const VK_V: u16 = 0x56;
pub const VK_W: u16 = 0x57;
pub const VK_X: u16 = 0x58;
pub const VK_Y: u16 = 0x59;
pub const VK_Z: u16 = 0x5A;

// 数字键 0-9
pub const VK_1: u16 = 0x31;
pub const VK_2: u16 = 0x32;
pub const VK_3: u16 = 0x33;
pub const VK_4: u16 = 0x34;
pub const VK_5: u16 = 0x35;
pub const VK_6: u16 = 0x36;
pub const VK_7: u16 = 0x37;
pub const VK_8: u16 = 0x38;
pub const VK_9: u16 = 0x39;
pub const VK_0: u16 = 0x30;

// 功能键 F1-F12
pub const VK_F1: u16 = 0x70;
pub const VK_F2: u16 = 0x71;

// ===== 鼠标速度补偿 =====
/// 基准鼠标速度（你的电脑上的设置）
const BASELINE_MOUSE_SPEED: i32 = 10;

/// 鼠标速度补偿系数缓存
static MOUSE_SPEED_MULTIPLIER: OnceLock<f64> = OnceLock::new();

/// 获取系统鼠标速度 (范围 1-20, 默认 10)
fn get_system_mouse_speed() -> i32 {
    let mut speed: i32 = 10;
    unsafe {
        // SPI_GETMOUSESPEED = 0x0070
        let _ = SystemParametersInfoA(
            windows::Win32::UI::WindowsAndMessaging::SPI_GETMOUSESPEED,
            0,
            Some(&mut speed as *mut i32 as *mut _),
            windows::Win32::UI::WindowsAndMessaging::SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
        );
    }
    speed
}

/// 获取鼠标速度补偿系数
fn get_mouse_speed_multiplier() -> f64 {
    *MOUSE_SPEED_MULTIPLIER.get_or_init(|| {
        let current_speed = get_system_mouse_speed();
        let multiplier = BASELINE_MOUSE_SPEED as f64 / current_speed as f64;
        println!(
            "[鼠标补偿] 系统速度: {}, 基准速度: {}, 补偿系数: {:.2}",
            current_speed, BASELINE_MOUSE_SPEED, multiplier
        );
        multiplier
    })
}

// ===== 鼠标操作 =====

/// 发送相对鼠标移动
pub fn send_relative(dx: i32, dy: i32) {
    let multiplier = get_mouse_speed_multiplier();
    let dx = (dx as f64 * multiplier) as i32;
    let dy = (dy as f64 * multiplier) as i32;

    let input = INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx,
                dy,
                mouseData: 0,
                dwFlags: MOUSEEVENTF_MOVE,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };

    unsafe {
        SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
    }
}

/// 鼠标左键点击 (SendInput 方式)
/// 注意：某些游戏可能屏蔽此方式，请使用 left_click_legacy
#[allow(dead_code)]
pub fn left_click() {
    let down = INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx: 0,
                dy: 0,
                mouseData: 0,
                dwFlags: MOUSEEVENTF_LEFTDOWN,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };

    let up = INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx: 0,
                dy: 0,
                mouseData: 0,
                dwFlags: MOUSEEVENTF_LEFTUP,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };

    unsafe {
        SendInput(&[down, up], std::mem::size_of::<INPUT>() as i32);
    }
}

/// 鼠标左键点击 (mouse_event 方式 - Legacy)
/// 使用更老的 mouse_event API，某些游戏只认这个
pub fn left_click_legacy() {
    unsafe {
        // MOUSEEVENTF_LEFTDOWN = 0x0002, MOUSEEVENTF_LEFTUP = 0x0004
        mouse_event(MOUSE_EVENT_FLAGS(0x0002), 0, 0, 0, 0);
        thread::sleep(Duration::from_millis(10));
        mouse_event(MOUSE_EVENT_FLAGS(0x0004), 0, 0, 0, 0);
    }
}

/// 移动鼠标到指定屏幕坐标
pub fn move_to(x: i32, y: i32) {
    use windows::Win32::UI::Input::KeyboardAndMouse::MOUSEEVENTF_ABSOLUTE;
    use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};

    let (screen_width, screen_height) = unsafe {
        (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN))
    };

    // 转换为绝对坐标 (0-65535)
    let abs_x = (x * 65535) / screen_width;
    let abs_y = (y * 65535) / screen_height;

    let input = INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx: abs_x,
                dy: abs_y,
                mouseData: 0,
                dwFlags: MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };

    unsafe {
        SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
    }
}

/// 移动鼠标并点击 (使用 legacy 方式)
pub fn click_at(x: i32, y: i32) {
    move_to(x, y);
    thread::sleep(Duration::from_millis(50));
    left_click_legacy();
}

/// 滚动方向
pub enum ScrollDirection {
    Up,
    Down,
}

/// 鼠标滚轮滚动
/// - direction: 滚动方向 (Up/Down)
/// - count: 滚动次数
/// - interval_secs: 每次滚动之间的间隔（秒）
pub fn mouse_scroll(direction: ScrollDirection, count: u32, interval_secs: f64) {
    // WHEEL_DELTA = 120，向上为正，向下为负
    let delta: i32 = match direction {
        ScrollDirection::Up => 120,
        ScrollDirection::Down => -120,
    };

    let dir_str = match direction {
        ScrollDirection::Up => "上",
        ScrollDirection::Down => "下",
    };

    println!("[鼠标滚动] 向{} 滚动 {} 次，间隔 {} 秒", dir_str, count, interval_secs);

    for i in 0..count {
        unsafe {
            // MOUSEEVENTF_WHEEL = 0x0800
            mouse_event(MOUSE_EVENT_FLAGS(0x0800), 0, 0, delta, 0);
        }

        if i < count - 1 {
            thread::sleep(Duration::from_secs_f64(interval_secs));
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

/// 按下指定键
pub fn key_down(vk: u16) {
    let input = INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(vk),
                wScan: 0,
                dwFlags: windows::Win32::UI::Input::KeyboardAndMouse::KEYBD_EVENT_FLAGS(0),
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };

    unsafe {
        SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
    }
}

/// 抬起指定键
pub fn key_up(vk: u16) {
    let input = INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(vk),
                wScan: 0,
                dwFlags: KEYEVENTF_KEYUP,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };

    unsafe {
        SendInput(&[input], std::mem::size_of::<INPUT>() as i32);
    }
}

/// 按住指定键持续一段时间
pub fn press_key(vk: u16, duration_secs: f64) {
    key_down(vk);
    println!("按下键 0x{:02X}，持续 {} 秒...", vk, duration_secs);
    thread::sleep(Duration::from_secs_f64(duration_secs));
    key_up(vk);
    println!("松开键 0x{:02X}", vk);
}

/// 点击（按下并立即抬起）指定键
pub fn tap_key(vk: u16) {
    key_down(vk);
    thread::sleep(Duration::from_millis(50));
    key_up(vk);
    println!("点击键 0x{:02X}", vk);
}

/// 按键序列动作类型
pub enum KeyAction {
    /// 按住指定时间（秒），0 表示只按下不松开
    Hold(u16, f64),
    /// 点击指定次数
    Tap(u16, u32),
    /// 松开指定键
    Release(u16),
}

/// 执行按键序列
pub fn press_key_sequence(actions: &[KeyAction]) {
    let mut held_keys: Vec<u16> = Vec::new();

    for (i, action) in actions.iter().enumerate() {
        match action {
            KeyAction::Hold(vk, duration) => {
                if *duration == 0.0 {
                    // 按住不松开
                    key_down(*vk);
                    held_keys.push(*vk);
                    println!("[{}] 按住 0x{:02X}", i + 1, vk);
                } else {
                    // 按住指定时间后松开
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

/// 从字符串获取虚拟键码
pub fn get_vk_code(key: &str) -> Option<u16> {
    let key = key.to_uppercase();
    let key = key.as_str();

    match key {
        // 字母 A-Z (0x41-0x5A)
        "A" => Some(0x41),
        "B" => Some(0x42),
        "C" => Some(0x43),
        "D" => Some(0x44),
        "E" => Some(0x45),
        "F" => Some(0x46),
        "G" => Some(0x47),
        "H" => Some(0x48),
        "I" => Some(0x49),
        "J" => Some(0x4A),
        "K" => Some(0x4B),
        "L" => Some(0x4C),
        "M" => Some(0x4D),
        "N" => Some(0x4E),
        "O" => Some(0x4F),
        "P" => Some(0x50),
        "Q" => Some(0x51),
        "R" => Some(0x52),
        "S" => Some(0x53),
        "T" => Some(0x54),
        "U" => Some(0x55),
        "V" => Some(0x56),
        "W" => Some(0x57),
        "X" => Some(0x58),
        "Y" => Some(0x59),
        "Z" => Some(0x5A),
        // 数字 0-9 (0x30-0x39)
        "0" => Some(0x30),
        "1" => Some(0x31),
        "2" => Some(0x32),
        "3" => Some(0x33),
        "4" => Some(0x34),
        "5" => Some(0x35),
        "6" => Some(0x36),
        "7" => Some(0x37),
        "8" => Some(0x38),
        "9" => Some(0x39),
        // 功能键
        "SPACE" => Some(VK_SPACE),
        "ENTER" => Some(VK_RETURN),
        "ESC" => Some(VK_ESCAPE),
        "TAB" => Some(VK_TAB),
        "SHIFT" => Some(VK_SHIFT),
        "CTRL" => Some(VK_CONTROL),
        "ALT" => Some(VK_ALT),
        // F1-F12
        "F1" => Some(0x70),
        "F2" => Some(0x71),
        "F3" => Some(0x72),
        "F4" => Some(0x73),
        "F5" => Some(0x74),
        "F6" => Some(0x75),
        "F7" => Some(0x76),
        "F8" => Some(0x77),
        "F9" => Some(0x78),
        "F10" => Some(0x79),
        "F11" => Some(0x7A),
        "F12" => Some(0x7B),
        _ => None,
    }
}
