# AutoEq Rust

耳机频响均衡的 Rust 重写版本，相比 Python 版快约 **2x**，导出为 DLL/SO 供 Java 等语言通过 FFI 调用。

## CLI 用法

```bash
cargo run --release -- equalize <测量文件.csv> [选项]
```

| 选项 | 说明 |
|------|------|
| `--target <file>` | 目标曲线文件 (CSV, 可选) |
| `--config <name>` | PEQ 配置名 (默认: `8_PEAKING_WITH_SHELVES`) |
| `--name <name>` | 测量名称 (默认取文件名) |

```bash
# 基础均衡
cargo run --release -- equalize ../test_file/harman2016.txt

# 指定目标曲线和 PEQ 配置
cargo run --release -- equalize ../test_file/harman2016.txt \
    --target ../test_file/5128DF1.5.txt \
    --config QUDELIX_5K

# 列出所有可用 PEQ 配置
cargo run --release -- configs

# 运行性能基准测试
cargo run --release -- bench
```

## 编译 DLL (Windows)

### 64-bit DLL

```powershell
cargo build --release
# 产物: target\release\autoeq.dll
```

### 32-bit DLL

```powershell
rustup target add i686-pc-windows-msvc
cargo build --release --target i686-pc-windows-msvc
# 产物: target\i686-pc-windows-msvc\release\autoeq.dll
```

### 减小 DLL 体积 (strip)

```powershell
# 使用 cargo-binstall 或手动安装 strip 工具
cargo install cargo-strip
cargo strip --release
```

## 编译 SO (Linux)

```bash
# glibc 2.28+ 兼容 (Debian Buster+)
cargo build --release
# 产物: target/release/libautoeq.so

# 静态链接 musl (glibc 版本无关)
rustup target add x86_64-unknown-linux-musl
cargo build --release --target x86_64-unknown-linux-musl
# 产物: target/x86_64-unknown-linux-musl/release/libautoeq.so
```

## 编译 dylib (macOS)

```bash
cargo build --release
# 产物: target/release/libautoeq.dylib
```

## 运行测试

```bash
cargo test
cargo test --release
```

## C FFI 接口

DLL/SO 导出以下 C ABI 函数，所有字符串使用 UTF-8 C 字符串：

| 函数 | 说明 |
|------|------|
| `autoeq_equalize_json(input) → json` | 完整均衡流水线，JSON 输入输出 |
| `autoeq_eq_by_range(input) → json` | 匹配 Python `/eq-by-range` 接口 |
| `autoeq_version() → string` | 返回版本号 |
| `autoeq_configs() → json` | 返回可用 PEQ 配置列表 |
| `autoeq_warmup()` | 预热 (触发 lazy_static 初始化) |
| `autoeq_free_string(ptr)` | 释放由以上函数返回的字符串 |

### `autoeq_equalize_json` 输入格式

```json
{
  "select": {
    "frequency": [20.0, 100.0, 1000.0, 10000.0],
    "raw": [-5.0, -3.0, 0.0, 2.0]
  },
  "target": {
    "frequency": [20.0, 100.0, 1000.0, 10000.0],
    "raw": [0.0, 0.0, 0.0, 0.0]
  },
  "config": "8_PEAKING_WITH_SHELVES",
  "fs": 44100.0,
  "preamp": 0.0
}
```

### `autoeq_eq_by_range` 输入格式

```json
{
  "select": {"frequency": [...], "raw": [...]},
  "target": {"frequency": [...], "raw": [...]},
  "eq_range": {"low": 20.0, "high": 20000.0},
  "config": "8_PEAKING_WITH_SHELVES",
  "fs": 44100.0,
  "max_filters": 10,
  "gain_range": {"low": -20.0, "high": 20.0},
  "q_range": {"low": 0.1, "high": 10.0}
}
```

## Java JNA 调用

详见 [java/README.md](java/README.md)。

```bash
# 1. 先编译 DLL
cargo build --release

# 2. 运行时指定库路径
java -Djna.library.path=./target/release -cp ... com.autoeq.Example
```

## 可用 PEQ 配置

- `8_PEAKING_WITH_SHELVES` (默认) — 1 低架 + 1 高架 + 8 峰值
- `10_PEAKING` — 10 个自由峰值滤波器
- `4_PEAKING_WITH_SHELVES` — 1 低架 + 1 高架 + 4 峰值
- `QUDELIX_5K`
- `MINIDSP_2X4HD`
- `MINIDSP_IL_DSP`
- `MOONDROP_FREE_DSP`
- `NEUTRON_MUSIC_PLAYER`
- `POWERAMP_EQUALIZER`
- `AUNBANDEQ`
- `SPOTIFY`
- `USB_AUDIO_PLAYER_PRO`
- `10_BAND_GRAPHIC_EQ`
- `31_BAND_GRAPHIC_EQ`
- `4_PEAKING_WITH_LOW_SHELF`
- `4_PEAKING_WITH_HIGH_SHELF`

## 项目结构

```
autoeq-rust/
├── src/
│   ├── main.rs               # CLI 入口
│   ├── lib.rs                # 库入口
│   ├── api.rs                # 高层 API
│   ├── frequency_response.rs # DSP 流水线
│   ├── peq.rs                # PEQ 优化器 (Nelder-Mead)
│   ├── dsp.rs                # DSP 工具 (Savitzky-Golay, 峰值检测等)
│   ├── csv.rs                # CSV 解析
│   ├── constants.rs          # 常量 + PEQ 配置定义
│   ├── ffi.rs                # C ABI 导出
│   ├── error.rs              # 错误类型
│   └── utils.rs              # 工具函数
├── java/                     # Java JNA 接口
└── Cargo.toml
```

## Python 对比

| 项目 | 8_PEAKING_WITH_SHELVES | 10_PEAKING |
|------|------------------------|------------|
| Python (scipy SLSQP) | ~259ms | ~232ms |
| Rust (Nelder-Mead, 并行) | ~131ms | ~111ms |
| **加速比** | **2.0x** | **2.1x** |

运行对比基准测试: `python ../bench_py_vs_rust.py`
