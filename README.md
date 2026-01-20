# nz-rust

逆战：未来 游戏自动化工具 (Rust 版)

## 快速开始

### 1. 安装 Rust

```bash
# Windows: 下载并运行 rustup-init.exe
# https://rustup.rs/
```

### 2. 下载 OCR 模型文件

在项目根目录创建 `models/` 文件夹，下载以下文件：

| 文件 | 下载地址 |
|------|---------|
| `ch_PP-OCRv4_det_infer.onnx` | [PaddleOCR Models](https://paddleocr.bj.bcebos.com/PP-OCRv4/chinese/ch_PP-OCRv4_det_infer.tar) |
| `ch_PP-OCRv4_rec_infer.onnx` | [PaddleOCR Models](https://paddleocr.bj.bcebos.com/PP-OCRv4/chinese/ch_PP-OCRv4_rec_infer.tar) |
| `ch_ppocr_mobile_v2.0_cls_infer.onnx` | [PaddleOCR Models](https://paddleocr.bj.bcebos.com/dygraph_v2.0/ch/ch_ppocr_mobile_v2.0_cls_infer.tar) |
| `ppocr_keys_v1.txt` | [GitHub](https://github.com/PaddlePaddle/PaddleOCR/blob/release/2.7/ppocr/utils/ppocr_keys_v1.txt) |

**注意**：下载的是 `.tar` 文件，需要解压后转换为 ONNX 格式，或直接寻找已转换好的 ONNX 模型。

目录结构：
```
nz-rust/
├── Cargo.toml
├── src/
│   ├── main.rs
│   ├── game.rs
│   ├── keys.rs
│   ├── ocr.rs
│   ├── screen.rs
│   └── stop_flag.rs
└── models/
    ├── ch_PP-OCRv4_det_infer.onnx
    ├── ch_PP-OCRv4_rec_infer.onnx
    ├── ch_ppocr_mobile_v2.0_cls_infer.onnx
    └── ppocr_keys_v1.txt
```

### 3. 编译运行

```bash
# 开发模式运行
cargo run

# 发布模式编译（优化后更快）
cargo build --release

# 运行发布版本
./target/release/nz-rust.exe
```

## 使用方法

| 热键 | 功能 |
|------|------|
| F1 | 开始游戏循环 |
| F2 | 停止所有任务 |
| Ctrl+C | 退出程序 |

## 游戏自动化流程

1. **启动游戏** - 查找窗口、设置位置、检测开始按钮
2. **购买陷阱** - 打开商店、购买破坏者/磁暴塔
3. **放置陷阱** - 在指定位置放置陷阱
4. **移动到安全点** - 移动到安全区域
5. **等待结束** - 等待游戏结束，自动截图

## 与 Python 版本的区别

| 特性 | Python 版 | Rust 版 |
|------|----------|--------|
| 启动速度 | 慢（~2秒） | 快（<0.1秒） |
| 可执行文件大小 | ~100MB | ~10MB |
| 内存占用 | 较高 | 较低 |
| 开发效率 | 高 | 中 |

## 注意事项

1. 游戏窗口必须为 1920x1080 分辨率
2. 窗口标题必须是 "逆战：未来  "（注意末尾两个空格）
3. 需要管理员权限运行（用于发送键鼠输入）

## 故障排除

### OCR 初始化失败
- 检查 `models/` 目录是否存在
- 检查模型文件是否完整

### 窗口未找到
- 确认游戏已启动
- 检查窗口标题是否正确

### 键鼠输入无效
- 以管理员身份运行
- 某些游戏可能有反作弊保护


# 运行所有测试（显示输出）
cargo test --release -- --nocapture

# 运行特定测试
cargo test --release test_ocr_custom_region -- --nocapture
cargo test --release test_ocr_fullscreen -- --nocapture
cargo test --release test_find_specific_text -- --nocapture