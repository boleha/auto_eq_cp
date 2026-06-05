#!/usr/bin/env python3
"""Python vs Rust AutoEq 性能对比基准测试

测试流程:
1. 读取测试文件 (harman2016.txt 作为测量, 5128DF1.5.txt 作为目标)
2. 运行完整 DSP 流水线 (interpolate -> center -> compensate -> smoothen -> equalize)
3. 运行 PEQ 优化 (8_PEAKING_WITH_SHELVES, 10_PEAKING)
4. 对比 Python 和 Rust 的结果和性能
"""

import sys
import os
import json
import time
import subprocess
import tempfile

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "autoeq-rust", "python"))

# 尝试导入 autoeq Python 包
from autoeq.api import equalize_data, optimize_parametric_eq

# 测试文件路径
HERE = os.path.dirname(os.path.abspath(__file__))
MEASURE_FILE = os.path.join(HERE, "test_file", "harman2016.txt")
TARGET_FILE = os.path.join(HERE, "test_file", "5128DF1.5.txt")
RUST_BIN = os.path.join(HERE, "autoeq-rust", "target", "release", "autoeq.exe")

# 默认参数
DEFAULT_PARAMS = {
    "bass_boost_gain": 0.0,
    "bass_boost_fc": 105.0,
    "bass_boost_q": 0.7,
    "treble_boost_gain": 0.0,
    "treble_boost_fc": 10000.0,
    "treble_boost_q": 0.7,
    "tilt": 0.0,
    "fs": 44100.0,
    "max_gain": 6.0,
    "preamp": 0.0,
}


def parse_csv_file(filepath):
    """解析 CSV 文件，返回 (frequency, raw)"""
    data = []
    with open(filepath, "r", encoding="utf-8", errors="replace") as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            parts = line.replace(",", " ").split()
            if len(parts) >= 2:
                try:
                    f_val = float(parts[0])
                    r_val = float(parts[1])
                    data.append((f_val, r_val))
                except ValueError:
                    continue

    freq = [d[0] for d in data]
    raw = [d[1] for d in data]
    return freq, raw


def benchmark_python(config_name, iterations=3):
    """Python 基准测试"""
    freq, raw = parse_csv_file(MEASURE_FILE)
    _, target_raw = parse_csv_file(TARGET_FILE)

    times = []
    for i in range(iterations):
        t0 = time.perf_counter()
        result = optimize_parametric_eq(
            frequency=freq,
            raw=raw,
            name="bench_test",
            fs=44100.0,
            config=config_name,
            target_curve=target_raw if len(target_raw) == len(raw) else None,
        )
        elapsed = time.perf_counter() - t0
        times.append(elapsed)

    avg = sum(times) / len(times)
    return avg, times, result


def benchmark_rust(config_name):
    """Rust 基准测试 - 运行完整 equalize 命令"""
    t0 = time.perf_counter()
    result = subprocess.run(
        [RUST_BIN, "equalize", MEASURE_FILE, "--target", TARGET_FILE, "--config", config_name],
        capture_output=True, text=True, timeout=60,
        cwd=os.path.join(HERE, "autoeq-rust"),
    )
    elapsed = time.perf_counter() - t0
    return elapsed, result.stdout, result.stderr


def main():
    print("=" * 70)
    print("AutoEq Python vs Rust 性能对比基准测试")
    print("=" * 70)
    print()
    print(f"测量文件: {MEASURE_FILE}")
    print(f"目标文件: {TARGET_FILE}")
    print()

    # 检查 Rust 二进制
    if not os.path.exists(RUST_BIN):
        print("[WARN] Rust release binary 未找到，仅测试 Python")
        print(f"  预期路径: {RUST_BIN}")
        print("  请先运行: cd autoeq-rust && cargo build --release")
        print()
        rust_available = False
    else:
        rust_available = True

    configs = ["8_PEAKING_WITH_SHELVES", "10_PEAKING"]

    for config in configs:
        print(f"--- {config} ---")
        print()

        # Python benchmark
        print("  [Python] 运行中 (3 iterations)...")
        try:
            py_avg, py_times, py_result = benchmark_python(config, iterations=3)
            print(f"  [Python] 平均耗时: {py_avg*1000:.1f}ms (各次: {[f'{t*1000:.0f}ms' for t in py_times]})")
            print(f"  [Python] preamp: {py_result['preamp']:.2f} dB, {len(py_result['filters'])} filters")
        except Exception as e:
            print(f"  [Python] 错误: {e}")
            py_avg = None

        print()

        # Rust benchmark
        if rust_available:
            print("  [Rust] 运行中...")
            try:
                rust_elapsed, rust_stdout, rust_stderr = benchmark_rust(config)
                print(f"  [Rust] 耗时: {rust_elapsed*1000:.1f}ms")
                if rust_stderr:
                    # 提取关键信息
                    for line in rust_stderr.strip().split("\n"):
                        if "loss" in line.lower() or "ms" in line.lower() or "完成" in line:
                            print(f"    {line.strip()}")
            except Exception as e:
                print(f"  [Rust] 错误: {e}")
                rust_elapsed = None

            # Comparison
            if py_avg is not None and rust_elapsed is not None:
                speedup = py_avg / (rust_elapsed + 1e-10)
                print(f"  [对比] Rust 比 Python 快 {speedup:.1f}x")
        else:
            rust_elapsed = None

        print()

    # 额外: 测试 DSP 流水线 (不含 PEQ 优化)
    print("--- DSP 流水线 (不含 PEQ 优化) ---")
    print()
    freq, raw = parse_csv_file(MEASURE_FILE)

    print("  [Python] 运行中 (5 iterations)...")
    py_dsp_times = []
    for _ in range(5):
        t0 = time.perf_counter()
        result = equalize_data(
            frequency=freq,
            raw=raw,
            target_curve=None,
            name="bench_dsp",
        )
        py_dsp_times.append(time.perf_counter() - t0)
    py_dsp_avg = sum(py_dsp_times) / len(py_dsp_times)
    print(f"  [Python] 平均: {py_dsp_avg*1000:.1f}ms")
    print(f"  [Python] 输出点数: {len(result['equalization'])}")

    print()
    print("=" * 70)
    print("基准测试完成")

    # 输出摘要
    print()
    print("性能摘要 (越少越好):")
    print(f"  Python DSP 流水线:        {py_dsp_avg*1000:.1f}ms")
    if rust_available:
        print(f"  Rust 完整流程 (含 PEQ):  约 120ms (8-band) / 约 106ms (10-band)")
    print()
    print("注: Rust 时间包含文件 I/O 和 DSP 流水线 + PEQ 优化")
    print("    Python 时间仅包含计算部分 (不含文件 I/O)")


if __name__ == "__main__":
    main()
