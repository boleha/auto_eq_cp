# AutoEq Java 接口

通过 JNA 调用 Rust 实现的 AutoEq 库。

## 依赖

- Java 8+
- JNA (Java Native Access) 5.x

### Maven

```xml
<dependency>
    <groupId>net.java.dev.jna</groupId>
    <artifactId>jna</artifactId>
    <version>5.14.0</version>
</dependency>
```

### Gradle

```groovy
implementation 'net.java.dev.jna:jna:5.14.0'
```

## 构建 Rust 动态库

```bash
cd autoeq-rust
cargo build --release
```

构建产物：
- Windows: `target/release/autoeq.dll`
- Linux: `target/release/libautoeq.so`
- macOS: `target/release/libautoeq.dylib`

## 使用方法

### 1. 将动态库放到 Java 库路径

将 `autoeq.dll`（或 `libautoeq.so`）放到以下位置之一：
- 项目根目录
- `PATH` 环境变量包含的目录（Windows）
- `LD_LIBRARY_PATH` 包含的目录（Linux）
- `-Djna.library.path=...` 指定的目录

### 2. Java 代码

```java
import com.autoeq.AutoEq;
import com.autoeq.EqualizeResult;
import com.autoeq.ProcessParams;

// 基本用法
double[] frequency = {20, 50, 100, 200, 500, 1000, 2000, 5000, 10000, 20000};
double[] raw = {5, 4, 3, 2, 1, 0, -1, -2, -3, -4};

EqualizeResult result = AutoEq.equalize(frequency, raw);
System.out.println(result);

// 自定义参数
ProcessParams params = new ProcessParams()
    .bassBoostGain(6.0)
    .tilt(0.5)
    .maxGain(12.0);

EqualizeResult result2 = AutoEq.equalize(
    frequency, raw, null, "My Headphone", "QUDELIX_5K", params
);

// 获取 PEQ 参数
if (result2.parametricEq != null) {
    System.out.println("Preamp: " + result2.parametricEq.preamp + " dB");
    for (var filter : result2.parametricEq.filters) {
        System.out.println(filter);
    }
}

// 获取 GraphicEQ 字符串
System.out.println(result2.graphicEq);
```

### 3. 运行时指定库路径

```bash
java -Djna.library.path=./target/release -cp ... com.autoeq.Example
```

## API 参考

### AutoEq.equalize(frequency, raw)
默认参数均衡。

### AutoEq.equalize(frequency, raw, target)
带目标曲线的均衡。

### AutoEq.equalize(frequency, raw, target, name, config, params)
完整参数控制。

### AutoEq.getVersion()
获取库版本号。

### AutoEq.getAvailableConfigs()
获取可用的 PEQ 配置名称列表。

## PEQ 配置名称

- `8_PEAKING_WITH_SHELVES` (默认)
- `4_PEAKING_WITH_SHELVES`
- `10_PEAKING`
- `QUDELIX_5K`
- `MINIDSP_2X4HD`
- `SPOTIFY`
- `AUNBANDEQ`
- 等共 16 种

## ProcessParams 参数

| 参数 | 默认值 | 说明 |
|------|--------|------|
| bassBoostGain | 0.0 | 低频增益 (dB) |
| bassBoostFc | 105.0 | 低频中心频率 (Hz) |
| bassBoostQ | 0.7 | 低频 Q 值 |
| trebleBoostGain | 0.0 | 高频增益 (dB) |
| trebleBoostFc | 10000.0 | 高频中心频率 (Hz) |
| trebleBoostQ | 0.7 | 高频 Q 值 |
| tilt | 0.0 | 倾斜 (dB/octave) |
| fs | 44100 | 采样率 |
| maxGain | 6.0 | 最大增益 (dB) |
| preamp | 0.0 | 前置放大 (dB) |
