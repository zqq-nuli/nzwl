//! Logitech 驱动层输入模块
//!
//! 通过 IbInputSimulator DLL 使用罗技驱动发送键盘鼠标输入
//! 相比 SendInput，驱动层输入更难被反作弊检测
//!
//! 需要安装 Logitech Gaming Software v9.02.65

use libloading::Library;
use std::ffi::c_void;
use std::path::Path;
use std::sync::OnceLock;
use std::thread;
use std::time::Duration;

// ===== FFI 类型定义 =====

#[repr(u32)]
#[derive(Debug, Clone, Copy)]
pub enum SendError {
    Success = 0,
    InvalidArgument = 1,
    LibraryNotFound = 2,
    LibraryLoadFailed = 3,
    LibraryError = 4,
    DeviceCreateFailed = 5,
    DeviceNotFound = 6,
    DeviceOpenFailed = 7,
}

#[repr(u32)]
#[derive(Debug, Clone, Copy)]
pub enum SendType {
    AnyDriver = 0,
    SendInput = 1,
    Logitech = 2,
    Razer = 3,
    DD = 4,
    MouClassInputInjection = 5,
    LogitechGHubNew = 6,
}

#[repr(u32)]
#[derive(Debug, Clone, Copy)]
pub enum MoveMode {
    Absolute = 0,
    Relative = 1,
}

#[repr(u32)]
#[derive(Debug, Clone, Copy)]
pub enum MouseButton {
    LeftDown = 0x02,
    LeftUp = 0x04,
    Left = 0x06,
    RightDown = 0x08,
    RightUp = 0x10,
    Right = 0x18,
    MiddleDown = 0x20,
    MiddleUp = 0x40,
    Middle = 0x60,
}

// ===== 函数签名类型 =====

type FnIbSendInit = unsafe extern "stdcall" fn(send_type: u32, flags: u32, argument: *mut c_void) -> u32;
type FnIbSendDestroy = unsafe extern "stdcall" fn();
type FnIbSendMouseMove = unsafe extern "stdcall" fn(x: i32, y: i32, mode: u32) -> bool;
type FnIbSendMouseClick = unsafe extern "stdcall" fn(button: u32) -> bool;
type FnIbSendMouseWheel = unsafe extern "stdcall" fn(movement: i32) -> bool;
type FnIbSendKeybdDown = unsafe extern "stdcall" fn(vk: u16) -> bool;
type FnIbSendKeybdUp = unsafe extern "stdcall" fn(vk: u16) -> bool;

// ===== 全局 DLL 实例 =====

struct LogitechDriver {
    _library: Library,
    send_init: FnIbSendInit,
    send_destroy: FnIbSendDestroy,
    mouse_move: FnIbSendMouseMove,
    mouse_click: FnIbSendMouseClick,
    mouse_wheel: FnIbSendMouseWheel,
    keybd_down: FnIbSendKeybdDown,
    keybd_up: FnIbSendKeybdUp,
    initialized: bool,
}

unsafe impl Send for LogitechDriver {}
unsafe impl Sync for LogitechDriver {}

static DRIVER: OnceLock<Result<LogitechDriver, String>> = OnceLock::new();

// ===== 初始化 =====

/// 获取 exe 所在目录
fn get_exe_dir() -> std::path::PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|p| p.to_path_buf()))
        .unwrap_or_else(|| std::path::PathBuf::from("."))
}

/// 加载 DLL 并获取函数指针
fn load_driver() -> Result<LogitechDriver, String> {
    let exe_dir = get_exe_dir();

    // 尝试多个可能的 DLL 路径（优先使用 exe 所在目录）
    let dll_paths: Vec<std::path::PathBuf> = vec![
        exe_dir.join("IbInputSimulator.dll"),
        std::path::PathBuf::from("IbInputSimulator.dll"),
        std::path::PathBuf::from("./IbInputSimulator.dll"),
        std::path::PathBuf::from("tools/IbInputSimulator_Release/IbInputSimulator.AHK2/IbInputSimulator.dll"),
    ];

    let mut last_error = String::new();

    for dll_path in &dll_paths {
        if !dll_path.exists() {
            continue;
        }

        match unsafe { Library::new(dll_path) } {
            Ok(lib) => {
                // 获取函数指针 - 先获取所有指针，再构建结构体
                let load_result: Result<LogitechDriver, String> = unsafe {
                    let send_init: FnIbSendInit = *lib.get(b"IbSendInit")
                        .map_err(|e| format!("Failed to load IbSendInit: {}", e))?;
                    let send_destroy: FnIbSendDestroy = *lib.get(b"IbSendDestroy")
                        .map_err(|e| format!("Failed to load IbSendDestroy: {}", e))?;
                    let mouse_move: FnIbSendMouseMove = *lib.get(b"IbSendMouseMove")
                        .map_err(|e| format!("Failed to load IbSendMouseMove: {}", e))?;
                    let mouse_click: FnIbSendMouseClick = *lib.get(b"IbSendMouseClick")
                        .map_err(|e| format!("Failed to load IbSendMouseClick: {}", e))?;
                    let mouse_wheel: FnIbSendMouseWheel = *lib.get(b"IbSendMouseWheel")
                        .map_err(|e| format!("Failed to load IbSendMouseWheel: {}", e))?;
                    let keybd_down: FnIbSendKeybdDown = *lib.get(b"IbSendKeybdDown")
                        .map_err(|e| format!("Failed to load IbSendKeybdDown: {}", e))?;
                    let keybd_up: FnIbSendKeybdUp = *lib.get(b"IbSendKeybdUp")
                        .map_err(|e| format!("Failed to load IbSendKeybdUp: {}", e))?;

                    Ok(LogitechDriver {
                        _library: lib,
                        send_init,
                        send_destroy,
                        mouse_move,
                        mouse_click,
                        mouse_wheel,
                        keybd_down,
                        keybd_up,
                        initialized: false,
                    })
                };

                match load_result {
                    Ok(driver) => {
                        println!("[Logitech] DLL loaded from: {}", dll_path.display());
                        return Ok(driver);
                    }
                    Err(e) => {
                        last_error = e;
                    }
                }
            }
            Err(e) => {
                last_error = format!("Failed to load {}: {}", dll_path.display(), e);
            }
        }
    }

    Err(format!("Could not load IbInputSimulator.dll: {}", last_error))
}

/// 初始化 Logitech 驱动
/// 必须在使用其他函数之前调用
pub fn init() -> Result<(), String> {
    let driver = DRIVER.get_or_init(|| {
        let mut driver = load_driver()?;

        // 初始化 Logitech 驱动
        let result = unsafe {
            (driver.send_init)(SendType::Logitech as u32, 0, std::ptr::null_mut())
        };

        if result != SendError::Success as u32 {
            return Err(format!("IbSendInit failed with error code: {}", result));
        }

        driver.initialized = true;
        println!("[Logitech] Driver initialized successfully");

        // 预热序列：发送几次虚拟移动来"唤醒"驱动
        // 这有助于解决首次运行时输入不生效的问题
        println!("[Logitech] Warming up driver...");
        for _ in 0..3 {
            unsafe {
                (driver.mouse_move)(0, 0, MoveMode::Relative as u32);
            }
            thread::sleep(Duration::from_millis(20));
        }
        println!("[Logitech] Driver ready");

        Ok(driver)
    });

    match driver {
        Ok(_) => Ok(()),
        Err(e) => Err(e.clone()),
    }
}

/// 获取已初始化的驱动
fn get_driver() -> Result<&'static LogitechDriver, String> {
    match DRIVER.get() {
        Some(Ok(driver)) if driver.initialized => Ok(driver),
        Some(Ok(_)) => Err("Driver not initialized".to_string()),
        Some(Err(e)) => Err(e.clone()),
        None => Err("Driver not loaded, call init() first".to_string()),
    }
}

/// 清理驱动资源
/// 注意：由于使用 OnceLock，这个函数在程序结束前只能调用一次
pub fn destroy() {
    if let Some(Ok(driver)) = DRIVER.get() {
        if driver.initialized {
            unsafe {
                (driver.send_destroy)();
            }
            println!("[Logitech] Driver destroyed");
        }
    }
}

// ===== 鼠标操作 =====

/// 相对移动鼠标
pub fn mouse_move_relative(dx: i32, dy: i32) -> Result<bool, String> {
    let driver = get_driver()?;
    let result = unsafe {
        (driver.mouse_move)(dx, dy, MoveMode::Relative as u32)
    };
    Ok(result)
}

/// 绝对移动鼠标 (屏幕坐标)
///
/// 通过 Logitech 驱动的相对移动实现，使用迭代修正确保精度。
/// 这样所有鼠标移动都通过驱动层，更好地规避反作弊。
pub fn mouse_move_absolute(x: i32, y: i32) -> Result<bool, String> {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

    let driver = get_driver()?;

    // 最多尝试 20 次修正
    for iteration in 0..20 {
        // 获取当前位置
        let mut current = POINT { x: 0, y: 0 };
        if unsafe { GetCursorPos(&mut current) }.is_err() {
            return Err("GetCursorPos failed".to_string());
        }

        // 计算差值
        let dx = x - current.x;
        let dy = y - current.y;

        // 如果已经足够接近目标（误差在 2 像素内），完成
        if dx.abs() <= 2 && dy.abs() <= 2 {
            return Ok(true);
        }

        // 使用保守的补偿策略，避免超调
        // 由于鼠标加速，实际移动通常是输入的 1.5-2.5 倍
        // 我们使用渐进式补偿：前几次迭代更保守，后面逐渐激进
        let base_divisor = if iteration < 3 {
            3.0  // 前几次非常保守，避免大幅超调
        } else if iteration < 6 {
            2.5
        } else if iteration < 10 {
            2.0
        } else {
            1.5  // 后期更激进，快速收敛
        };

        // 计算补偿后的移动量
        let mut move_dx = (dx as f64 / base_divisor).round() as i32;
        let mut move_dy = (dy as f64 / base_divisor).round() as i32;

        // 限制单次移动量，避免一次性移动太远
        const MAX_MOVE: i32 = 200;
        move_dx = move_dx.clamp(-MAX_MOVE, MAX_MOVE);
        move_dy = move_dy.clamp(-MAX_MOVE, MAX_MOVE);

        // 确保至少移动 1 像素（如果需要移动的话）
        if move_dx == 0 && dx != 0 {
            move_dx = dx.signum();
        }
        if move_dy == 0 && dy != 0 {
            move_dy = dy.signum();
        }

        // 执行移动
        unsafe {
            (driver.mouse_move)(move_dx, move_dy, MoveMode::Relative as u32);
        }

        // 短暂延迟让移动生效
        std::thread::sleep(std::time::Duration::from_millis(8));
    }

    Ok(true)
}

/// 鼠标左键点击
/// 使用分离的 Down/Up 以兼容 UE4 等游戏引擎
pub fn left_click() -> Result<bool, String> {
    let driver = get_driver()?;

    // 先发送一个微小的相对移动来"激活"鼠标位置
    // 这有助于 UE4 等引擎注册鼠标悬停状态
    unsafe {
        (driver.mouse_move)(0, 0, MoveMode::Relative as u32);
    }
    thread::sleep(Duration::from_millis(16)); // 约 1 帧

    // 按下
    unsafe {
        (driver.mouse_click)(MouseButton::LeftDown as u32);
    }

    // 等待一段时间，模拟真实点击
    thread::sleep(Duration::from_millis(50));

    // 抬起
    let result = unsafe {
        (driver.mouse_click)(MouseButton::LeftUp as u32)
    };

    // 点击后等待一帧
    thread::sleep(Duration::from_millis(16));

    Ok(result)
}

/// 鼠标左键按下
pub fn left_down() -> Result<bool, String> {
    let driver = get_driver()?;
    let result = unsafe {
        (driver.mouse_click)(MouseButton::LeftDown as u32)
    };
    Ok(result)
}

/// 鼠标左键抬起
pub fn left_up() -> Result<bool, String> {
    let driver = get_driver()?;
    let result = unsafe {
        (driver.mouse_click)(MouseButton::LeftUp as u32)
    };
    Ok(result)
}

/// 鼠标右键点击
pub fn right_click() -> Result<bool, String> {
    let driver = get_driver()?;
    let result = unsafe {
        (driver.mouse_click)(MouseButton::Right as u32)
    };
    Ok(result)
}

/// 鼠标滚轮
/// movement > 0 向上滚动，< 0 向下滚动
pub fn mouse_wheel(movement: i32) -> Result<bool, String> {
    let driver = get_driver()?;
    let result = unsafe {
        (driver.mouse_wheel)(movement)
    };
    Ok(result)
}

/// 移动鼠标到指定位置并点击
/// 增加足够的延迟让游戏引擎注册新位置
pub fn click_at(x: i32, y: i32) -> Result<(), String> {
    mouse_move_absolute(x, y)?;
    // 等待游戏引擎更新鼠标位置（UE4 通常需要 1-2 帧）
    thread::sleep(Duration::from_millis(100));
    left_click()?;
    Ok(())
}

// ===== 键盘操作 =====

/// 按下键
pub fn key_down(vk: u16) -> Result<bool, String> {
    let driver = get_driver()?;
    let result = unsafe {
        (driver.keybd_down)(vk)
    };
    Ok(result)
}

/// 抬起键
pub fn key_up(vk: u16) -> Result<bool, String> {
    let driver = get_driver()?;
    let result = unsafe {
        (driver.keybd_up)(vk)
    };
    Ok(result)
}

/// 点击键（按下并抬起）
pub fn tap_key(vk: u16) -> Result<(), String> {
    key_down(vk)?;
    thread::sleep(Duration::from_millis(50));
    key_up(vk)?;
    Ok(())
}

/// 按住键一段时间
pub fn press_key(vk: u16, duration_secs: f64) -> Result<(), String> {
    key_down(vk)?;
    thread::sleep(Duration::from_secs_f64(duration_secs));
    key_up(vk)?;
    Ok(())
}

// ===== 兼容 keys.rs 的接口 =====

/// 视角向左转（相对移动）
pub fn move_left(value: i32) -> Result<(), String> {
    mouse_move_relative(-value, 0)?;
    println!("[Logitech] 向左 {}", value);
    Ok(())
}

/// 视角向右转
pub fn move_right(value: i32) -> Result<(), String> {
    mouse_move_relative(value, 0)?;
    println!("[Logitech] 向右 {}", value);
    Ok(())
}

/// 视角向上
pub fn move_up(value: i32) -> Result<(), String> {
    mouse_move_relative(0, -value)?;
    println!("[Logitech] 向上 {}", value);
    Ok(())
}

/// 视角向下
pub fn move_down(value: i32) -> Result<(), String> {
    mouse_move_relative(0, value)?;
    println!("[Logitech] 向下 {}", value);
    Ok(())
}

/// 滚动方向
pub enum ScrollDirection {
    Up,
    Down,
}

/// 鼠标滚轮滚动
pub fn scroll(direction: ScrollDirection, count: u32, interval_secs: f64) -> Result<(), String> {
    let delta: i32 = match direction {
        ScrollDirection::Up => 120,
        ScrollDirection::Down => -120,
    };

    for i in 0..count {
        mouse_wheel(delta)?;
        if i < count - 1 {
            thread::sleep(Duration::from_secs_f64(interval_secs));
        }
    }
    Ok(())
}

// ===== 测试 =====

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_init() {
        match init() {
            Ok(_) => println!("Init succeeded"),
            Err(e) => println!("Init failed: {}", e),
        }
    }
}
