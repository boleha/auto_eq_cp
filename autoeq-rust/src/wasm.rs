use wasm_bindgen::prelude::*;

use crate::api::{self, ProcessParams};
use crate::constants::PEQ_CONFIGS;

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
