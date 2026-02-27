# 策略编写指南

本文档面向想要为新地图/难度编写自动化策略的开发者。

## 目录

- [快速开始](#快速开始)
- [文件结构](#文件结构)
- [坐标系统与分辨率适配](#坐标系统与分辨率适配)
- [常用函数参考](#常用函数参考)
- [完整示例](#完整示例)
- [调试技巧](#调试技巧)

---

## 快速开始

添加一个新地图只需 3 步：

1. 在 `src/game/` 下新建模块文件（如 `my_map.rs`）
2. 在 `src/game/mod.rs` 中注册模块和地图信息
3. 编写波次函数

完成后重新编译，GUI 下拉框中就会出现新地图。

---

## 文件结构

### 1. 新建模块文件

创建 `src/game/my_map.rs`，基本骨架：

```rust
//! 我的地图 - 困难难度
//!
//! 装备顺序（购买顺序决定热键）:
//!   防空导弹 → 4键, 自修复磁暴塔 → 5键, 破坏者 → 6键, 修理站 → 7键

use anyhow::Result;
use std::thread;
use std::time::Duration;

use super::common::{
    buy_traps, place_trap, start_game_with_difficulty,
    wait_for_game_end, wait_gold, wait_wave,
};
use crate::input::{click_at, move_to, left_click, tap_key};
use crate::screen::{scale_x, scale_y};  // 坐标缩放函数
use crate::stop_flag::should_stop;

/// 开始游戏
pub fn start_game() -> Result<()> {
    start_game_with_difficulty("困难")
}

/// 波次 1
pub fn wave_1() -> Result<()> {
    if should_stop() { return Ok(()); }
    println!("[我的地图] === 波次 1 ===");

    buy_traps()?;
    wait_gold(2500)?;
    place_trap(scale_x(800), scale_y(400), "5")?;

    Ok(())
}

/// 波次 2
pub fn wave_2() -> Result<()> {
    if should_stop() { return Ok(()); }
    println!("[我的地图] === 波次 2 ===");

    wait_gold(5000)?;
    place_trap(scale_x(600), scale_y(300), "6")?;

    Ok(())
}

/// 执行所有波次
pub fn run_all_waves() -> Result<()> {
    wave_1()?;

    wait_wave(2)?;   // ← 阻塞！直到后台监控检测到波次 >= 2 才继续
    wave_2()?;

    wait_for_game_end()?;
    Ok(())
}
```

> **`wait_wave()` 是如何工作的？**
>
> `run_all_waves()` 并不是"一口气执行完所有波次"。`wait_wave(n)` 内部是一个循环，
> 持续读取后台监控线程的 `current_wave()` 值（通过 OCR 实时识别屏幕上的波次数字），
> **只有当游戏画面上的波次数 >= n 时才会往下执行**。
>
> 实际执行流程：
> ```
> wave_1()       → 执行波次1的放置/操作
> wait_wave(2)   → 阻塞等待...游戏画面显示"波次2"后才放行
> wave_2()       → 执行波次2的放置/操作
> wait_wave(3)   → 阻塞等待...游戏画面显示"波次3"后才放行
> wave_3()       → ...
> ```
>
> 所以每个波次函数一定是在游戏真正推进到对应波次后才被调用的。

### 2. 注册到 `mod.rs`

编辑 `src/game/mod.rs`：

```rust
pub mod building_inferno;
pub mod common;
pub mod training_hard;
pub mod my_map;           // ← 添加模块声明

// ...

pub fn available_maps() -> Vec<MapInfo> {
    vec![
        // ... 已有地图 ...
        MapInfo {
            name: "我的地图",
            difficulty: "困难",
            start_fn: my_map::start_game,
            waves_fn: my_map::run_all_waves,
        },
    ]
}
```

### 3. 波次函数的规则

- **每个波次一个函数**：`wave_1()`、`wave_2()`、`wave_3_boss()` 等
- **入口必须检查停止信号**：每个波次函数第一行写 `if should_stop() { return Ok(()); }`
- **`run_all_waves()` 用 `wait_wave()` 串联**：波次之间用 `wait_wave(n)` 等待后台监控检测到波次变化
- **调试时注释波次**：可以在 `run_all_waves()` 中注释掉前面的波次，只跑后面的

---

## 坐标系统与分辨率适配

这是最重要的部分。程序需要在不同分辨率的屏幕上运行，所以**坐标不能写死**。

### 核心概念

程序提供了两套坐标缩放函数，对应两个"基准分辨率"：

| 函数 | 基准分辨率 | 定义位置 | 适用场景 |
|------|-----------|---------|---------|
| `scale_x()` / `scale_y()` | 1920x1080 | `screen.rs` 的 `BASE_WIDTH` / `BASE_HEIGHT` | 通用基准，common.rs 内部使用 |
| `dev_x()` / `dev_y()` | 3840x2160 | `screen.rs` 的 `DEV_WIDTH` / `DEV_HEIGHT` | 开发者在自己屏幕上录制坐标 |

**缩放原理**：你填写的坐标 ÷ 基准分辨率 × 实际屏幕分辨率 = 最终坐标

```
最终坐标 = 填写坐标 × (实际分辨率 / 基准分辨率)
```

### 选择哪套函数？

#### 方案 A：使用 `scale_x()` / `scale_y()`（推荐新手）

坐标以 **1920x1080** 为基准。无论你的屏幕是什么分辨率，都**按 1080p 来填写坐标**，程序自动缩放。

```rust
use crate::screen::{scale_x, scale_y};

// 在 1080p 下，屏幕中心是 (960, 540)
click_at(scale_x(960), scale_y(540));

// 放置陷阱，坐标按 1080p 填写
place_trap(scale_x(800), scale_y(400), "5")?;

// 批量放置（place_traps 内部已调用 scale_x/scale_y，传入 1080p 坐标即可）
place_traps(&[(800, 400), (900, 400), (1000, 400)], "5")?;
```

如果你的屏幕是 4K (3840x2160)，你需要自己把截图上看到的坐标**除以 2**再填写。

#### 方案 B：使用 `dev_x()` / `dev_y()`（推荐：直接用你屏幕上的坐标）

坐标以 `DEV_WIDTH` x `DEV_HEIGHT`（默认 3840x2160）为基准。**你在自己屏幕上看到什么坐标就填什么坐标**，程序自动换算到实际运行分辨率。

```rust
use crate::screen::{dev_x, dev_y};

// 4K 屏幕上截图看到按钮在 (2906, 443)，直接填
click_at(dev_x(2906), dev_y(443));

// 4K 屏幕上截图看到陷阱位置在 (1600, 800)
place_trap(dev_x(1600), dev_y(800), "5")?;
```

### 修改基准分辨率（关键！）

**如果你的屏幕不是 4K**，需要修改 `src/screen.rs` 中的 `DEV_WIDTH` / `DEV_HEIGHT`：

```rust
// src/screen.rs

// ===== 修改这里！改成你的屏幕分辨率 =====

/// 开发环境分辨率（策略文件中的坐标以此为基准）
pub const DEV_WIDTH: u32 = 3840;   // ← 改成你的屏幕宽度
pub const DEV_HEIGHT: u32 = 2160;  // ← 改成你的屏幕高度
```

修改后，你就可以用 `dev_x()` / `dev_y()` 直接填写你屏幕上的坐标了：

| 你的屏幕 | `DEV_WIDTH` | `DEV_HEIGHT` | 填写坐标示例（屏幕中心） |
|---------|-------------|-------------|----------------------|
| 4K (3840x2160) | 3840 | 2160 | `dev_x(1920), dev_y(1080)` |
| 2K (2560x1440) | 2560 | 1440 | `dev_x(1280), dev_y(720)` |
| 1080p (1920x1080) | 1920 | 1080 | `dev_x(960), dev_y(540)` |

**修改 `DEV_WIDTH` / `DEV_HEIGHT` 后，所有使用 `dev_x()` / `dev_y()` 的策略都会自动适配其他分辨率。**

> **注意**：`BASE_WIDTH` / `BASE_HEIGHT` (1920x1080) 是 `common.rs` 内部逻辑使用的，一般不需要修改。

### 坐标获取方法

1. **使用 OCR 测试工具**：`cargo run --release --bin ocr-test`，在工具中框选区域可以看到像素坐标
2. **Windows 自带截图**：按 Win+Shift+S 截图，用画图工具打开，鼠标悬停看左下角坐标
3. **游戏内 F12**：部分游戏支持截图并显示坐标

### 各函数的坐标类型速查

| 函数 | 期望坐标类型 | 说明 |
|------|------------|------|
| `click_at(x, y)` | **实际屏幕坐标** | 需要自己调用 `scale_x/dev_x` 转换 |
| `move_to(x, y)` | **实际屏幕坐标** | 需要自己调用 `scale_x/dev_x` 转换 |
| `place_trap(x, y, key)` | **实际屏幕坐标** | 需要自己调用 `scale_x/dev_x` 转换 |
| `place_traps(&[(x,y)], key)` | **1080p 基准坐标** | 内部已调用 `scale_x/scale_y` |
| `place_trap_at(x, y, vk)` | **实际屏幕坐标** | 旧接口，需要自己转换 |
| `ocr_screen(x, y, w, h, ...)` | **实际屏幕坐标** | 用 `scale_region()` 或 `full_screen_region()` |
| `scale_region(x, y, w, h)` | **1080p 基准坐标** | 返回实际屏幕坐标元组 |

---

## 常用函数参考

### 游戏流程控制

```rust
// 等待金币达到指定数额（后台监控线程持续检测）
wait_gold(5000)?;

// 等待波次推进到指定值
wait_wave(3)?;

// 检查是否应该停止（用户点了停止按钮）
if should_stop() { return Ok(()); }

// 等待游戏结束（检测结算界面）
wait_for_game_end()?;
```

### 陷阱操作

```rust
// 购买陷阱 - 默认顺序：防空导弹(4), 自修复磁暴塔(5), 破坏者(6), 修理站(7)
buy_traps()?;

// 自定义购买顺序（顺序决定热键：第1个→4键, 第2个→5键, ...）
buy_traps_ordered(&["天网", "自修复磁暴塔", "天启", "防空导弹"])?;

// 放置陷阱：按热键 + 点击坐标（坐标需要是实际屏幕坐标）
place_trap(scale_x(800), scale_y(400), "5")?;   // 方案A: 1080p 基准
place_trap(dev_x(1600), dev_y(800), "5")?;       // 方案B: 开发分辨率基准

// 批量放置（坐标为 1080p 基准，内部自动缩放）
place_traps(&[
    (800, 400),
    (900, 400),
    (1000, 400),
], "5")?;

// 升级陷阱（长按热键 3 秒）
upgrade_trap("5")?;
```

### 鼠标操作

```rust
use crate::input::{click_at, move_to, left_click};
use crate::screen::{scale_x, scale_y, dev_x, dev_y};

// 移动鼠标到指定位置并点击（坐标需转换）
click_at(scale_x(960), scale_y(540));       // 方案A
click_at(dev_x(1920), dev_y(1080));         // 方案B

// 只移动不点击
move_to(scale_x(960), scale_y(540));

// 在当前位置点击
left_click();
```

### 键盘操作

```rust
use crate::input::{tap_key, press_key, key_down, key_up, VK_SPACE, VK_W, VK_G};

// 单击按键
tap_key(VK_G);           // 按一下 G（开始波次）

// 长按按键（秒）
press_key(VK_W, 3.0);   // 按住 W 3秒（前进）
press_key(VK_SPACE, 2.0); // 按住空格 2秒（跳过动画）

// 手动按下/松开（组合操作用）
key_down(VK_W);
tap_key(VK_SPACE);       // 按住 W 的同时跳跃
thread::sleep(Duration::from_millis(500));
key_up(VK_W);
```

### OCR 识别

```rust
use crate::ocr::{ocr_screen, find_text_contains};
use crate::screen::{full_screen_region, scale_region};

// 全屏 OCR
let (fx, fy, fw, fh) = full_screen_region();
let results = ocr_screen(fx, fy, fw, fh, false, false)?;

// 局部 OCR（1080p 基准坐标 → 自动缩放）
let (rx, ry, rw, rh) = scale_region(84, 230, 393, 61);
let results = ocr_screen(rx, ry, rw, rh, false, false)?;

// 在结果中查找文字
if let Some(r) = find_text_contains(&results, "炼狱") {
    let (cx, cy) = r.center();     // 返回的是实际屏幕坐标，可直接使用
    click_at(cx, cy);
}
```

### 视角转动

```rust
use crate::input::send_relative;

// 相对移动鼠标（用于转动视角，不受分辨率影响）
send_relative(2237, 0);   // 水平转约 180 度
send_relative(-284, 0);   // 水平转约 -35 度
send_relative(0, 100);    // 垂直向下转
```

> `send_relative` 是相对移动，与分辨率无关，不需要 `scale_x`/`dev_x` 转换。

---

## 完整示例

以下是一个实战策略的骨架（大厦炼狱），展示了典型的编写模式：

```rust
//! 大厦 - 炼狱难度

use anyhow::Result;
use std::thread;
use std::time::Duration;

use super::common::*;
use crate::input::{click_at, press_key, VK_SPACE};
use crate::ocr::{find_text_contains, ocr_screen};
use crate::screen::{dev_x, dev_y, full_screen_region};
use crate::stop_flag::should_stop;

// ===== 陷阱热键（购买顺序决定） =====
const TIANWANG: &str = "4";     // 第1个购买 → 4键
const CIBAO: &str = "5";       // 第2个购买 → 5键
const TIANQI: &str = "6";      // 第3个购买 → 6键
const FANGKONG: &str = "7";    // 第4个购买 → 7键

const EQUIPPED_TRAPS: &[&str] = &["天网", "自修复磁暴塔", "天启", "防空导弹"];

// ===== 开始游戏（自定义开局逻辑） =====
pub fn start_game() -> Result<()> {
    let hwnd = find_game_window().context("未找到游戏窗口")?;
    setup_window(hwnd)?;

    let (fx, fy, fw, fh) = full_screen_region();
    let results = ocr_screen(fx, fy, fw, fh, false, true)?;

    // 用 OCR 找按钮并点击（返回的坐标是实际屏幕坐标，直接用）
    if let Some(r) = find_text_contains(&results, "炼狱") {
        let (cx, cy) = r.center();
        click_at(cx, cy);
    }

    // 用 dev_x/dev_y 点击固定位置按钮（坐标从 4K 截图上量的）
    click_at(dev_x(2665), dev_y(1772));

    wait_wave(1)?;
    Ok(())
}

// ===== 波次函数 =====
pub fn wave_1() -> Result<()> {
    if should_stop() { return Ok(()); }

    // 购买陷阱
    buy_traps_ordered(EQUIPPED_TRAPS)?;

    // 等金币攒够再放
    wait_gold(3000)?;
    place_trap(dev_x(1500), dev_y(900), CIBAO)?;
    place_trap(dev_x(1600), dev_y(900), CIBAO)?;

    Ok(())
}

pub fn wave_2() -> Result<()> {
    if should_stop() { return Ok(()); }

    wait_gold(8000)?;
    place_trap(dev_x(1400), dev_y(800), TIANQI)?;

    // 升级磁暴塔
    upgrade_trap(CIBAO)?;

    Ok(())
}

// ===== 串联所有波次 =====
pub fn run_all_waves() -> Result<()> {
    wave_1()?;
    wait_wave(2)?;
    wave_2()?;
    // ...更多波次...
    wait_for_game_end()?;
    Ok(())
}
```

---

## 调试技巧

### 1. 只跑某几个波次

在 `run_all_waves()` 中注释掉不需要的波次：

```rust
pub fn run_all_waves() -> Result<()> {
    // wave_1()?;           // 跳过波次1
    // wait_wave(2)?;
    // wave_2()?;           // 跳过波次2

    wait_wave(3)?;          // 从波次3开始跑
    wave_3()?;

    wait_wave(4)?;
    wave_4()?;

    wait_for_game_end()?;
    Ok(())
}
```

### 2. 用 OCR 测试工具校准坐标

```bash
cargo run --release --bin ocr-test
```

在 OCR 测试工具中：
- 拖动框选区域可以看到精确的像素坐标
- 勾选"小区域预处理"可以测试金币/波次数字识别效果
- 复制区域坐标到策略代码中

### 3. 开启调试输出

`common.rs` 中 `IS_DEBUG = true` 会打印所有 OCR 识别结果。

### 4. 编译与运行

```bash
# 必须用 --release（debug 模式 MNN 库有 CRT 兼容问题）
cargo build --release
cargo run --release
```
