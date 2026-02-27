# 逆战未来挂机框架

## 项目说明

本项目是《逆战：未来》自动化挂机框架的 Rust 实现。

当前包含：
- GUI 主程序（地图选择、启动/停止、实时波次/金币监控、日志面板）
- OCR 调试工具（GUI）
- 波次函数策略系统（每个地图/难度一个模块，按波次编写逻辑）
- 后台监控系统（波次 + 金币 OCR 轮询）
- 多分辨率自动适配

## 环境要求

- Windows 10/11（x64）
- Rust 1.75+（建议使用最新 stable）
- 游戏窗口标题为 `逆战：未来  `（末尾 2 个空格）
- 建议以管理员身份运行

可选依赖（仅 Logitech 模式需要）：
- `IbInputSimulator.dll`
- Logitech Gaming Software `v9.02.65`

## 目录结构

```text
nz-rust/
├─ src/
│  ├─ main.rs              # GUI 主程序
│  ├─ monitor.rs           # 后台波次/金币监控
│  ├─ game/
│  │  ├─ mod.rs            # 地图注册（available_maps）
│  │  ├─ common.rs         # 通用函数（购买、放置、等待等）
│  │  ├─ training_hard.rs  # 训练基地 - 困难
│  │  └─ building_inferno.rs # 大厦 - 炼狱
│  ├─ ocr.rs               # OCR 引擎封装
│  ├─ screen.rs            # 截图 + 分辨率缩放
│  ├─ input.rs             # 输入抽象层
│  ├─ keys.rs              # SendInput 后端
│  ├─ logitech.rs          # Logitech 驱动后端
│  └─ stop_flag.rs         # 停止信号
├─ models/                 # OCR 模型文件（MNN 格式）
├─ docs/
│  └─ strategy-guide.md    # 策略编写指南
└─ images/                 # 参考图片
```

## 如何运行

### 1. 运行 GUI 主程序

```powershell
cargo run --release
```

在 GUI 中选择地图和难度，点击"开始"即可。热键：
- `F1`：开始
- `F2`：停止

### 2. 运行 OCR 调试工具

```powershell
cargo run --release --bin ocr-test
```

## 编写策略

想为新地图/难度编写自动化策略？请阅读：

**[策略编写指南 (docs/strategy-guide.md)](docs/strategy-guide.md)**

指南包含：
- 3 步添加新地图的完整流程
- 坐标系统与多分辨率适配详解（`scale_x`/`scale_y` vs `dev_x`/`dev_y`）
- 如何修改基准分辨率让你直接用自己屏幕上的坐标
- 所有常用函数的参考和示例
- 调试技巧

## 如何测试

说明：本项目测试中有一部分依赖真实屏幕/OCR模型/输入环境，不适合纯 CI 无头环境。

### 1. 先做测试编译（推荐）

```powershell
cargo test --release --no-run
```

### 2. 运行全部测试（会调用屏幕截图与OCR）

```powershell
cargo test --release -- --nocapture
```

### 3. 运行指定测试

```powershell
cargo test --release test_init_ocr -- --nocapture
cargo test --release test_capture_region -- --nocapture
cargo test --release test_ocr_custom_region -- --nocapture
```

## 安全说明

仓库已通过 `.gitignore` 默认忽略敏感文件（`.env*`、`*.pem`、`*.key` 等）和打包产物（`dist/`、`*.zip`）。


## 免责声明与使用限制

本项目仅用于开发测试与学习研究，请勿用于任何违法违规、破坏公平或侵犯他人权益的场景。

使用本项目即表示你同意：
- 遵守当地法律法规与目标平台服务条款
- 对你的使用行为和后果自行负责（包括但不限于账号风险）
- 不将本项目用于恶意或未授权用途

完整说明见：`DISCLAIMER.md`
