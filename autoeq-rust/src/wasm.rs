use wasm_bindgen::prelude::*;

use crate::api::{self, ProcessParams};
use crate::constants::PEQ_CONFIGS;
use crate::frequency_response::FrequencyResponse;

/// Initialize panic hook for better error messages in browser console.
/// Call this once before any other WASM function.
#[wasm_bindgen]
pub fn init_panic_hook() {
    console_error_panic_hook::set_once();
}

// ===== Input types (mirrors JSON) =====

#[derive(serde::Deserialize)]
struct EqualizeInput {
    frequency: Vec<f64>,
    raw: Vec<f64>,
    #[serde(default)]
    target_curve: Option<Vec<f64>>,
    #[serde(default = "default_name")]
    name: String,
    #[serde(default)]
    bass_boost_gain: f64,
    #[serde(default = "default_bass_fc")]
    bass_boost_fc: f64,
    #[serde(default = "default_bass_q")]
    bass_boost_q: f64,
    #[serde(default)]
    treble_boost_gain: f64,
    #[serde(default = "default_treble_fc")]
    treble_boost_fc: f64,
    #[serde(default = "default_treble_q")]
    treble_boost_q: f64,
    #[serde(default)]
    tilt: f64,
    #[serde(default = "default_fs")]
    fs: f64,
    #[serde(default = "default_max_gain")]
    max_gain: f64,
}

#[derive(serde::Deserialize)]
struct OptimizeInput {
    frequency: Vec<f64>,
    raw: Vec<f64>,
    #[serde(default = "default_name")]
    name: String,
    #[serde(default = "default_config")]
    config: String,
    #[serde(default)]
    target_curve: Option<Vec<f64>>,
    #[serde(default = "default_fs")]
    fs: f64,
    #[serde(default)]
    bass_boost_gain: f64,
    #[serde(default = "default_bass_fc")]
    bass_boost_fc: f64,
    #[serde(default = "default_bass_q")]
    bass_boost_q: f64,
    #[serde(default)]
    treble_boost_gain: f64,
    #[serde(default = "default_treble_fc")]
    treble_boost_fc: f64,
    #[serde(default = "default_treble_q")]
    treble_boost_q: f64,
    #[serde(default)]
    tilt: f64,
    #[serde(default = "default_max_gain")]
    max_gain: f64,
}

// ===== Default helpers =====

fn default_name() -> String { "headphone".into() }
fn default_config() -> String { "8_PEAKING_WITH_SHELVES".into() }
fn default_fs() -> f64 { 44100.0 }
fn default_max_gain() -> f64 { 6.0 }
fn default_bass_fc() -> f64 { 105.0 }
fn default_bass_q() -> f64 { 0.7 }
fn default_treble_fc() -> f64 { 10000.0 }
fn default_treble_q() -> f64 { 0.7 }

// ===== WASM exports =====

/// 完整 DSP 均衡流水线 (interpolate → center → compensate → smoothen → equalize)
/// 输入 JSON: { frequency, raw, target_curve?, name?, ... }
/// 输出 JSON: { frequency, raw, smoothed, equalization, target, error }
#[wasm_bindgen]
pub fn equalize_data_js(input_json: &str) -> String {
    let input: EqualizeInput = match serde_json::from_str(input_json) {
        Ok(v) => v,
        Err(e) => return serde_json::json!({ "error": format!("Parse error: {}", e) }).to_string(),
    };

    let params = ProcessParams {
        bass_boost_gain: input.bass_boost_gain,
        bass_boost_fc: input.bass_boost_fc,
        bass_boost_q: input.bass_boost_q,
        treble_boost_gain: input.treble_boost_gain,
        treble_boost_fc: input.treble_boost_fc,
        treble_boost_q: input.treble_boost_q,
        tilt: input.tilt,
        fs: input.fs,
        max_gain: input.max_gain,
        preamp: 0.0,
        // 平滑度等其余参数用默认值
        ..ProcessParams::default()
    };

    match api::equalize_data(
        &input.frequency,
        &input.raw,
        input.target_curve.as_deref(),
        &input.name,
        &params,
    ) {
        Ok(result) => serde_json::to_string(&result).unwrap_or_else(|e| {
            serde_json::json!({ "error": format!("Serialize error: {}", e) }).to_string()
        }),
        Err(e) => serde_json::json!({ "error": format!("{}", e) }).to_string(),
    }
}

/// PEQ 参数优化 (含 DSP 流水线 + Nelder-Mead 优化)
/// 输入 JSON: { frequency, raw, config?, target_curve?, ... }
/// 输出 JSON: { preamp, filters: [{ type, fc, gain, q }] }
#[wasm_bindgen]
pub fn optimize_parametric_eq_js(input_json: &str) -> String {
    let input: OptimizeInput = match serde_json::from_str(input_json) {
        Ok(v) => v,
        Err(e) => return serde_json::json!({ "error": format!("Parse error: {}", e) }).to_string(),
    };

    let params = ProcessParams {
        bass_boost_gain: input.bass_boost_gain,
        bass_boost_fc: input.bass_boost_fc,
        bass_boost_q: input.bass_boost_q,
        treble_boost_gain: input.treble_boost_gain,
        treble_boost_fc: input.treble_boost_fc,
        treble_boost_q: input.treble_boost_q,
        tilt: input.tilt,
        fs: input.fs,
        max_gain: input.max_gain,
        preamp: 0.0,
        // 平滑度等其余参数用默认值
        ..ProcessParams::default()
    };

    match api::optimize_parametric_eq(
        &input.frequency,
        &input.raw,
        &input.name,
        &params,
        &input.config,
        input.target_curve.as_deref(),
    ) {
        Ok(result) => serde_json::to_string(&result).unwrap_or_else(|e| {
            serde_json::json!({ "error": format!("Serialize error: {}", e) }).to_string()
        }),
        Err(e) => serde_json::json!({ "error": format!("{}", e) }).to_string(),
    }
}

/// 获取所有可用 PEQ 配置名称列表
#[wasm_bindgen]
pub fn get_configs_js() -> String {
    let configs: Vec<&str> = PEQ_CONFIGS.keys().copied().collect();
    serde_json::to_string(&configs).unwrap_or_else(|_| "[]".into())
}

/// 获取版本号
#[wasm_bindgen]
pub fn version_js() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

// ===== eq-by-range 输入/输出结构 =====

#[derive(serde::Deserialize)]
struct EqByRangeInput {
    select: FfiFrequencyData,
    target: FfiFrequencyData,
    eq_range: Option<FfiRange>,
    fs: Option<f64>,
    config: Option<String>,
    preamp: Option<f64>,
    max_filters: Option<usize>,
    gain_range: Option<FfiRange>,
    q_range: Option<FfiRange>,
    // 平滑度控制参数 —— 与 ffi.rs::EqRangeInput 保持一致（对齐 Python 原生 AutoEq 接口）
    #[serde(default)]
    window_size: Option<f64>,
    #[serde(default)]
    treble_window_size: Option<f64>,
    #[serde(default)]
    treble_f_lower: Option<f64>,
    #[serde(default)]
    treble_f_upper: Option<f64>,
    #[serde(default)]
    treble_gain_k: Option<f64>,
    #[serde(default)]
    max_gain: Option<f64>,
    #[serde(default)]
    max_slope: Option<f64>,
    #[serde(default)]
    tilt: Option<f64>,
    #[serde(default)]
    bass_boost_gain: Option<f64>,
    #[serde(default)]
    bass_boost_fc: Option<f64>,
    #[serde(default)]
    bass_boost_q: Option<f64>,
    #[serde(default)]
    treble_boost_gain: Option<f64>,
    #[serde(default)]
    treble_boost_fc: Option<f64>,
    #[serde(default)]
    treble_boost_q: Option<f64>,
    #[serde(default)]
    min_mean_error: Option<bool>,
}

#[derive(serde::Deserialize)]
struct FfiFrequencyData {
    frequency: Vec<f64>,
    raw: Vec<f64>,
}

#[derive(serde::Deserialize, Clone)]
struct FfiRange {
    low: Option<f64>,
    high: Option<f64>,
}

#[derive(serde::Serialize)]
struct EqByRangeOutput {
    preamp: f64,
    filters: Vec<FilterResult>,
    eq_range: FfiRangeOut,
    gain_range: Option<FfiRangeOut>,
    q_range: Option<FfiRangeOut>,
    fs: f64,
    max_filters: Option<usize>,
}

#[derive(serde::Serialize)]
struct FilterResult {
    #[serde(rename = "type")]
    filter_type: String,
    fc: f64,
    gain: f64,
    q: f64,
}

#[derive(serde::Serialize)]
struct FfiRangeOut {
    low: f64,
    high: f64,
}

/**
 * eq-by-range：按频率范围生成参数均衡器 (匹配 Python /eq-by-range 行为)
 *
 * 输入:
 * {
 *   "select":   { "frequency": [...], "raw": [...] },
 *   "target":   { "frequency": [...], "raw": [...] },
 *   "eq_range":  { "low": 20, "high": 20000 },      // optional
 *   "config":    "8_PEAKING_WITH_SHELVES",           // optional
 *   "fs":        44100,                               // optional
 *   "preamp":    0.0,                                 // optional
 *   "max_filters": 10,                                // optional
 *   "gain_range": { "low": -12, "high": 12 },        // optional
 *   "q_range":   { "low": 0.3, "high": 5 }           // optional
 * }
 *
 * 输出:
 * {
 *   "preamp": -1.2,
 *   "filters": [{ "type": "Peaking", "fc": 1000, "gain": 0.5, "q": 1.0 }],
 *   "eq_range": { "low": 20, "high": 20000 },
 *   "gain_range": { "low": -12, "high": 12 },
 *   "q_range": { "low": 0.3, "high": 5 },
 *   "fs": 44100,
 *   "max_filters": 10
 * }
 */
#[wasm_bindgen]
pub fn eq_by_range_js(input_json: &str) -> String {
    let input: EqByRangeInput = match serde_json::from_str(input_json) {
        Ok(v) => v,
        Err(e) => return serde_json::json!({ "error": format!("Parse error: {}", e) }).to_string(),
    };

    let fs = input.fs.unwrap_or(44100.0);
    let config_name = input.config.as_deref().unwrap_or("8_PEAKING_WITH_SHELVES");
    let preamp = input.preamp.unwrap_or(0.0);

    // 目标曲线可能分辨率不同，需要插值到 select 的频率轴
    let target_raw_aligned = match align_target_to_select(&input.target, &input.select, fs) {
        Ok(r) => r,
        Err(e) => return serde_json::json!({ "error": e }).to_string(),
    };

    // 平滑度等参数：入参没给就用 Rust 默认值（与 ffi.rs 逻辑一致）
    let defaults = ProcessParams::default();
    let params = ProcessParams {
        fs,
        preamp,
        window_size: input.window_size.unwrap_or(defaults.window_size),
        treble_window_size: input.treble_window_size.unwrap_or(defaults.treble_window_size),
        treble_f_lower: input.treble_f_lower.unwrap_or(defaults.treble_f_lower),
        treble_f_upper: input.treble_f_upper.unwrap_or(defaults.treble_f_upper),
        treble_gain_k: input.treble_gain_k.unwrap_or(defaults.treble_gain_k),
        max_gain: input.max_gain.unwrap_or(defaults.max_gain),
        max_slope: input.max_slope.unwrap_or(defaults.max_slope),
        tilt: input.tilt.unwrap_or(defaults.tilt),
        bass_boost_gain: input.bass_boost_gain.unwrap_or(defaults.bass_boost_gain),
        bass_boost_fc: input.bass_boost_fc.unwrap_or(defaults.bass_boost_fc),
        bass_boost_q: input.bass_boost_q.unwrap_or(defaults.bass_boost_q),
        treble_boost_gain: input.treble_boost_gain.unwrap_or(defaults.treble_boost_gain),
        treble_boost_fc: input.treble_boost_fc.unwrap_or(defaults.treble_boost_fc),
        treble_boost_q: input.treble_boost_q.unwrap_or(defaults.treble_boost_q),
        min_mean_error: input.min_mean_error.unwrap_or(defaults.min_mean_error),
    };

    // PEQ 优化
    let peq_result = match api::optimize_parametric_eq(
        &input.select.frequency,
        &input.select.raw,
        "select",
        &params,
        config_name,
        Some(&target_raw_aligned),
    ) {
        Ok(r) => r,
        Err(e) => return serde_json::json!({ "error": format!("PEQ error: {}", e) }).to_string(),
    };

    // 收集所有滤波器 (peq_result.filters 是 Vec<FilterResult> 结构体)
    let mut all_filters: Vec<FilterResult> = peq_result.filters.iter().map(|f| FilterResult {
        filter_type: format!("{:?}", f.filter_type),
        fc: f.fc,
        gain: f.gain,
        q: f.q,
    }).collect();

    // 按 eq_range 过滤 (频率范围)
    if let Some(ref range) = input.eq_range {
        if let Some(low) = range.low {
            all_filters.retain(|f| f.fc >= low);
        }
        if let Some(high) = range.high {
            all_filters.retain(|f| f.fc <= high);
        }
    }

    // 按 gain_range 过滤
    if let Some(ref range) = input.gain_range {
        if let Some(low) = range.low {
            all_filters.retain(|f| f.gain.abs() >= low);
        }
        if let Some(high) = range.high {
            all_filters.retain(|f| f.gain.abs() <= high);
        }
    }

    // 按 q_range 过滤
    if let Some(ref range) = input.q_range {
        if let Some(low) = range.low {
            all_filters.retain(|f| f.q >= low);
        }
        if let Some(high) = range.high {
            all_filters.retain(|f| f.q <= high);
        }
    }

    // 按 max_filters 限制数量 (按增益绝对值降序，保留前 N 个)
    if let Some(max) = input.max_filters {
        if all_filters.len() > max {
            all_filters.sort_by(|a, b| b.gain.abs().partial_cmp(&a.gain.abs()).unwrap());
            all_filters.truncate(max);
        }
    }

    // preamp = -max_gain - headroom (与 FFI 版保持一致, headroom=0.2)
    let max_gain = peq_result.filters.iter().map(|f| f.gain.abs()).fold(0.0_f64, f64::max);
    let actual_preamp = -max_gain.max(0.0) - 0.2;

    let eq_range_low = input.eq_range.as_ref().and_then(|r| r.low).unwrap_or(20.0);
    let eq_range_high = input.eq_range.as_ref().and_then(|r| r.high).unwrap_or(20000.0);

    let output = EqByRangeOutput {
        preamp: actual_preamp,
        filters: all_filters,
        eq_range: FfiRangeOut { low: eq_range_low, high: eq_range_high },
        gain_range: input.gain_range.map(|r| FfiRangeOut {
            low: r.low.unwrap_or(0.0),
            high: r.high.unwrap_or(f64::INFINITY),
        }),
        q_range: input.q_range.map(|r| FfiRangeOut {
            low: r.low.unwrap_or(0.0),
            high: r.high.unwrap_or(f64::INFINITY),
        }),
        fs,
        max_filters: input.max_filters,
    };

    serde_json::to_string(&output).unwrap_or_else(|e| {
        serde_json::json!({ "error": format!("Serialize error: {}", e) }).to_string()
    })
}

/// 把 target 曲线插值到 select 的频率轴上, 以便两条曲线能对位补偿.
fn align_target_to_select(
    target: &FfiFrequencyData,
    select: &FfiFrequencyData,
    _fs: f64,
) -> Result<Vec<f64>, String> {
    use crate::constants::{DEFAULT_STEP, DEFAULT_F_MIN, DEFAULT_F_MAX};

    let mut target_fr = FrequencyResponse::new(
        "target",
        target.frequency.clone(),
        target.raw.clone(),
    ).map_err(|e| format!("Target create error: {}", e))?;

    let _ = target_fr.interpolate(Some(&select.frequency), DEFAULT_STEP, DEFAULT_F_MIN, DEFAULT_F_MAX);
    let _ = target_fr.center(1000.0);

    Ok(target_fr.raw)
}
