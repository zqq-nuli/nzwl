//! nz-rust: 逆战：未来 游戏自动化工具 (Rust 版)
//!
//! 热键控制:
//! - F1: 开始游戏循环
//! - F2: 停止所有任务

mod game;
mod input;
mod keys;
mod logitech;
mod ocr;
mod screen;
mod stop_flag;
mod strategy;
mod strategy_executor;

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;
use std::thread;
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    RegisterHotKey, UnregisterHotKey, HOT_KEY_MODIFIERS, MOD_NOREPEAT,
};
use windows::Win32::UI::WindowsAndMessaging::{GetMessageW, MSG, WM_HOTKEY};

use crate::game::{main_game_loop, start_game};
use crate::stop_flag::{request_stop, reset_stop, should_stop};

/// 热键 ID
const HOTKEY_F1: i32 = 1;
const HOTKEY_F2: i32 = 2;

/// 虚拟键码
const VK_F1: u32 = 0x70;
const VK_F2: u32 = 0x71;

/// 游戏是否正在运行
static GAME_RUNNING: AtomicBool = AtomicBool::new(false);

/// 策略文件路径（启动时设置，运行期间只读）
static STRATEGY_PATH: OnceLock<PathBuf> = OnceLock::new();

/// 解析 --strategy 参数
fn parse_strategy_arg() -> Option<PathBuf> {
    let args: Vec<String> = std::env::args().collect();
    for i in 0..args.len() {
        if args[i] == "--strategy" {
            if let Some(path) = args.get(i + 1) {
                return Some(PathBuf::from(path));
            }
        }
    }
    None
}

fn main() {
    println!("=== 逆战：未来 自动化脚本 (Rust 版) ===");
    println!("按 F1 开始游戏");
    println!("按 F2 停止所有任务");
    println!("按 Ctrl+C 退出\n");

    // 解析 --strategy 参数
    let strategy_path = parse_strategy_arg();
    if let Some(ref path) = strategy_path {
        println!("使用策略文件: {}", path.display());
        match strategy::Strategy::load(path) {
            Ok(s) => {
                println!("策略加载成功: {} (难度: {})", s.meta.name, s.meta.difficulty);
                let _ = STRATEGY_PATH.set(path.clone());
            }
            Err(e) => {
                eprintln!("策略文件加载失败: {}", e);
                return;
            }
        }
    }

    // 初始化 OCR 引擎
    println!("正在初始化 OCR 引擎...");
    match ocr::init_ocr() {
        Ok(_) => println!("OCR 引擎初始化完成"),
        Err(e) => {
            eprintln!("OCR 初始化失败: {}", e);
            eprintln!("请确保模型文件存在于 models/ 目录");
            eprintln!("\n按 Enter 退出...");
            let mut buf = String::new();
            let _ = std::io::stdin().read_line(&mut buf);
            return;
        }
    }

    // 初始化输入系统 - 尝试使用 Logitech 驱动，失败则回退到 SendInput
    println!("正在初始化输入系统...");
    match input::init(input::InputBackend::Logitech) {
        Ok(_) => println!("输入系统初始化完成 (Logitech 驱动)\n"),
        Err(e) => {
            println!("Logitech 驱动初始化失败: {}", e);
            println!("回退到 SendInput 模式");
            let _ = input::init(input::InputBackend::SendInput);
            println!("输入系统初始化完成 (SendInput)\n");
        }
    }

    // 注册热键
    unsafe {
        let result1 = RegisterHotKey(
            HWND::default(),
            HOTKEY_F1,
            HOT_KEY_MODIFIERS(MOD_NOREPEAT.0),
            VK_F1,
        );
        let result2 = RegisterHotKey(
            HWND::default(),
            HOTKEY_F2,
            HOT_KEY_MODIFIERS(MOD_NOREPEAT.0),
            VK_F2,
        );

        if result1.is_err() || result2.is_err() {
            eprintln!("注册热键失败！可能需要管理员权限，或热键已被其他程序占用。");
            return;
        }
    }

    println!("热键已注册，等待输入...\n");

    // Windows 消息循环
    unsafe {
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, HWND::default(), 0, 0).as_bool() {
            if msg.message == WM_HOTKEY {
                let hotkey_id = msg.wParam.0 as i32;
                match hotkey_id {
                    HOTKEY_F1 => handle_f1_pressed(),
                    HOTKEY_F2 => handle_f2_pressed(),
                    _ => {}
                }
            }
        }

        // 清理热键
        let _ = UnregisterHotKey(HWND::default(), HOTKEY_F1);
        let _ = UnregisterHotKey(HWND::default(), HOTKEY_F2);
    }
}

/// F1 按下 - 开始游戏循环
fn handle_f1_pressed() {
    println!("[F1] 按下");

    if GAME_RUNNING.load(Ordering::SeqCst) {
        println!("[WARN] 游戏正在运行，请先按 F2 停止");
        return;
    }

    // 重置停止标志
    reset_stop();
    GAME_RUNNING.store(true, Ordering::SeqCst);

    // 在新线程中运行游戏循环
    thread::spawn(|| {
        println!("[START] 游戏线程已启动");
        game_loop();
        GAME_RUNNING.store(false, Ordering::SeqCst);
        println!("[STOP] 游戏线程已结束");
    });
}

/// F2 按下 - 停止所有任务
fn handle_f2_pressed() {
    println!("[F2] 按下");
    request_stop();
    println!("\n[STOP] 已请求停止所有任务，正在安全退出...");
    GAME_RUNNING.store(false, Ordering::SeqCst);
}

/// 游戏主循环
fn game_loop() {
    let mut round = 0;
    const MAX_ROUNDS: i32 = 100;

    // 如果有策略文件，使用策略执行器
    let use_strategy = STRATEGY_PATH.get().is_some();

    while round < MAX_ROUNDS && !should_stop() {
        println!("\n[LOOP] 开始第 {} 轮", round + 1);

        if use_strategy {
            let path = STRATEGY_PATH.get().unwrap();
            // 每轮重新加载策略（允许运行中修改 JSON）
            match strategy::Strategy::load(path) {
                Ok(s) => {
                    if let Err(e) = strategy_executor::start_game_with_strategy(&s) {
                        eprintln!("[ERROR] 策略执行失败: {}", e);
                        if should_stop() {
                            break;
                        }
                    }
                }
                Err(e) => {
                    eprintln!("[ERROR] 策略加载失败: {}", e);
                    break;
                }
            }
        } else {
            // 原有硬编码逻辑
            if let Err(e) = start_game() {
                eprintln!("[ERROR] startGame 失败: {}", e);
                if should_stop() {
                    break;
                }
            }

            if should_stop() {
                println!("[STOP] startGame 后检测到停止信号");
                break;
            }

            if let Err(e) = main_game_loop() {
                eprintln!("[ERROR] main 失败: {}", e);
                if should_stop() {
                    break;
                }
            }
        }

        if should_stop() {
            println!("[STOP] 检测到停止信号");
            break;
        }

        round += 1;
        println!("[LOOP] 第 {} 轮完成", round);
    }

    println!("[STOP] 游戏循环已结束，共完成 {} 轮", round);
}
