# 逆战未来挂机框架（开发中）

## 项目说明

本项目是《逆战：未来》自动化挂机框架的 Rust 实现，当前处于开发中状态。

当前包含：
- 主程序（全局热键触发自动流程）
- 策略执行器（支持 JSON 策略）
- 地图策略编辑器（GUI）
- OCR 调试工具（GUI）
- Logitech 驱动输入测试工具

## 环境要求

- Windows 10/11（x64）
- Rust 1.75+（建议使用最新 stable）
- 游戏窗口建议为 `1920x1080`
- 建议以管理员身份运行（全局热键/输入注入更稳定）

可选依赖（仅 Logitech 模式需要）：
- `IbInputSimulator.dll`
- Logitech Gaming Software `v9.02.65`

## 目录结构

```text
nz-rust/
├─ src/
├─ strategies/
├─ models/
│  ├─ ch_PP-OCRv4_det_infer.mnn
│  ├─ ch_PP-OCRv4_rec_infer.mnn
│  └─ ppocr_keys_v4.txt
└─ scripts/
   └─ package_release.ps1
```

## 如何运行

### 1. 直接运行主程序

```powershell
cargo run --release
```

主程序热键：
- `F1`：开始循环
- `F2`：请求停止
- `Ctrl + C`：退出程序

### 2. 指定策略文件运行

```powershell
cargo run --release -- --strategy strategies/hero.json
```

### 3. 运行调试工具

```powershell
# OCR + 输入调试 GUI
cargo run --release --bin ocr-test

# 地图策略编辑器 GUI
cargo run --release --bin map-editor

# Logitech 驱动连通性测试
cargo run --release --bin logitech-test

# 鼠标绝对定位/点击测试
cargo run --release --bin mouse_test
```

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

## 如何打包 Release（主程序 + 调试工具）

执行：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\\scripts\\package_release.ps1
```

脚本会自动：
- 编译 `release` 全部二进制
- 收集主程序和调试工具到 `dist/release/<时间戳>/`
- 复制 `models/` 与 `strategies/`
- 自动生成压缩包 `dist/nzwl-release-<时间戳>.zip`

## 安全说明（避免提交私钥）

仓库已通过 `.gitignore` 默认忽略：
- `.env*`
- 常见证书/密钥文件（`*.pem`, `*.key`, `*.p12`, `*.pfx` 等）
- 本地临时与打包产物（`dist/`, `*.zip`, `tmp_*`）

如需提交新文件，建议先执行：

```powershell
git status --short
```

确认无敏感文件后再 `git add`。


## 免责声明与使用限制

本项目仅用于开发测试与学习研究，请勿用于任何违法违规、破坏公平或侵犯他人权益的场景。

使用本项目即表示你同意：
- 遵守当地法律法规与目标平台服务条款
- 对你的使用行为和后果自行负责（包括但不限于账号风险）
- 不将本项目用于恶意或未授权用途

完整说明见：`DISCLAIMER.md`
