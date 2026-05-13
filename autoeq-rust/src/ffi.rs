use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr;

use crate::api::{self, ProcessParams};
use crate::peq::PeqResult;

// ===== eq-by-range 输入结构 (匹配 Python /eq-by-range 接口) =====

#[derive(serde::Deserialize)]
struct EqRangeInput {
    select: FfiFrequencyData,
    target: FfiFrequencyData,
    eq_range: Option<FfiRange>,
    fs: Option<f64>,
    config: Option<String>,
    preamp: Option<f64>,
    max_filters: Option<usize>,
    gain_range: Option<FfiRange>,
    q_range: Option<FfiRange>,
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

// ===== eq-by-range 输出结构 =====

#[derive(serde::Serialize)]
struct EqRangeOutput {
    preamp: f64,
    filters: Vec<FilterOutput>,
    eq_range: FfiRangeOut,
    gain_range: Option<FfiRangeOut>,
    q_range: Option<FfiRangeOut>,
    fs: f64,
    max_filters: Option<usize>,
}

#[derive(serde::Serialize)]
struct FilterOutput {
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

// ===== 通用输入/输出 =====

#[derive(serde::Deserialize)]
struct FfiInput {
    frequency: Vec<f64>,
    raw: Vec<f64>,
    target: Option<Vec<f64>>,
    name: Option<String>,
    config: Option<String>,
    params: Option<FfiParams>,
}

#[derive(serde::Deserialize)]
struct FfiParams {
    bass_boost_gain: Option<f64>,
    bass_boost_fc: Option<f64>,
    bass_boost_q: Option<f64>,
    treble_boost_gain: Option<f64>,
    treble_boost_fc: Option<f64>,
    treble_boost_q: Option<f64>,
    tilt: Option<f64>,
    fs: Option<f64>,
    max_gain: Option<f64>,
    preamp: Option<f64>,
}

#[derive(serde::Serialize)]
struct FfiOutput {
    success: bool,
    error: Option<String>,
    name: Option<String>,
    frequency: Option<Vec<f64>>,
    raw: Option<Vec<f64>>,
    smoothed: Option<Vec<f64>>,
    equalization: Option<Vec<f64>>,
    target: Option<Vec<f64>>,
    error_curve: Option<Vec<f64>>,
    parametric_eq: Option<PeqResult>,
    graphic_eq: Option<String>,
}

impl FfiParams {
    fn to_process_params(&self) -> ProcessParams {
        let default = ProcessParams::default();
        ProcessParams {
            bass_boost_gain: self.bass_boost_gain.unwrap_or(default.bass_boost_gain),
            bass_boost_fc: self.bass_boost_fc.unwrap_or(default.bass_boost_fc),
            bass_boost_q: self.bass_boost_q.unwrap_or(default.bass_boost_q),
            treble_boost_gain: self.treble_boost_gain.unwrap_or(default.treble_boost_gain),
            treble_boost_fc: self.treble_boost_fc.unwrap_or(default.treble_boost_fc),
            treble_boost_q: self.treble_boost_q.unwrap_or(default.treble_boost_q),
            tilt: self.tilt.unwrap_or(default.tilt),
            fs: self.fs.unwrap_or(default.fs),
            max_gain: self.max_gain.unwrap_or(default.max_gain),
            preamp: self.preamp.unwrap_or(default.preamp),
        }
    }
}

/// eq-by-range: 匹配 Python /eq-by-range 接口
/// 输入格式与 Java AutoEqRequest 完全一致
///
/// # Safety
/// 返回的指针必须用 autoeq_free_string 释放
#[no_mangle]
pub unsafe extern "C" fn autoeq_eq_by_range(input: *const c_char) -> *mut c_char {
    let input_str = match unsafe { CStr::from_ptr(input) }.to_str() {
        Ok(s) => s,
        Err(e) => return error_response(&format!("Invalid UTF-8: {}", e)),
    };

    let req: EqRangeInput = match serde_json::from_str(input_str) {
        Ok(v) => v,
        Err(e) => return error_response(&format!("Invalid JSON: {}", e)),
    };

    let fs = req.fs.unwrap_or(44100.0);
    let config_name = req.config.as_deref().unwrap_or("8_PEAKING_WITH_SHELVES");
    let preamp = req.preamp.unwrap_or(0.0);

    // 关键：target 的频率轴与 select 不同，必须先插值对齐
    let mut target_fr = match crate::frequency_response::FrequencyResponse::new(
        "target", req.target.frequency.clone(), req.target.raw.clone()
    ) {
        Ok(t) => t,
        Err(e) => return error_response(&format!("Target create error: {}", e)),
    };
    let _ = target_fr.interpolate(Some(&req.select.frequency), crate::constants::DEFAULT_STEP,
        crate::constants::DEFAULT_F_MIN, crate::constants::DEFAULT_F_MAX);
    let target_raw_aligned = target_fr.raw;

    let params = ProcessParams { fs, preamp, ..ProcessParams::default() };

    let eq_result = match api::equalize_data(
        &req.select.frequency,
        &req.select.raw,
        Some(&target_raw_aligned),
        "select",
        &params,
    ) {
        Ok(r) => r,
        Err(e) => return error_response(&format!("Equalize error: {}", e)),
    };

    // PEQ 优化
    let peq_result = match api::optimize_parametric_eq(
        &req.select.frequency,
        &req.select.raw,
        "select",
        &params,
        config_name,
        Some(&target_raw_aligned),
    ) {
        Ok(r) => r,
        Err(e) => return error_response(&format!("PEQ error: {}", e)),
    };

    // 收集所有滤波器
    let mut all_filters: Vec<FilterOutput> = peq_result.filters.iter().map(|f| FilterOutput {
        filter_type: format!("{:?}", f.filter_type),
        fc: f.fc,
        gain: f.gain,
        q: f.q,
    }).collect();

    // 按 eq_range 过滤 (频率范围)
    if let Some(ref range) = req.eq_range {
        if let Some(low) = range.low {
            all_filters.retain(|f| f.fc >= low);
        }
        if let Some(high) = range.high {
            all_filters.retain(|f| f.fc <= high);
        }
    }

    // 按 gain_range 过滤
    if let Some(ref range) = req.gain_range {
        if let Some(low) = range.low {
            all_filters.retain(|f| f.gain.abs() >= low);
        }
        if let Some(high) = range.high {
            all_filters.retain(|f| f.gain.abs() <= high);
        }
    }

    // 按 q_range 过滤
    if let Some(ref range) = req.q_range {
        if let Some(low) = range.low {
            all_filters.retain(|f| f.q >= low);
        }
        if let Some(high) = range.high {
            all_filters.retain(|f| f.q <= high);
        }
    }

    // 按 max_filters 限制数量 (保留增益最大的)
    if let Some(max) = req.max_filters {
        if all_filters.len() > max {
            all_filters.sort_by(|a, b| b.gain.abs().partial_cmp(&a.gain.abs()).unwrap());
            all_filters.truncate(max);
        }
    }

    let max_gain = peq_result.filters.iter().map(|f| f.gain.abs()).fold(0.0_f64, f64::max);
    let actual_preamp = -max_gain.max(0.0) - 0.2; // PREAMP_HEADROOM

    let eq_range_low = req.eq_range.as_ref().and_then(|r| r.low).unwrap_or(20.0);
    let eq_range_high = req.eq_range.as_ref().and_then(|r| r.high).unwrap_or(20000.0);

    let output = EqRangeOutput {
        preamp: actual_preamp,
        filters: all_filters,
        eq_range: FfiRangeOut { low: eq_range_low, high: eq_range_high },
        gain_range: req.gain_range.map(|r| FfiRangeOut {
            low: r.low.unwrap_or(0.0),
            high: r.high.unwrap_or(f64::INFINITY),
        }),
        q_range: req.q_range.map(|r| FfiRangeOut {
            low: r.low.unwrap_or(0.0),
            high: r.high.unwrap_or(f64::INFINITY),
        }),
        fs,
        max_filters: req.max_filters,
    };

    success_response(&output)
}

/// 通用均衡接口 (完整输出)
///
/// # Safety
/// 返回的指针必须用 autoeq_free_string 释放
#[no_mangle]
pub unsafe extern "C" fn autoeq_equalize_json(input: *const c_char) -> *mut c_char {
    let input_str = match unsafe { CStr::from_ptr(input) }.to_str() {
        Ok(s) => s,
        Err(e) => return error_json(&format!("Invalid UTF-8: {}", e)),
    };

    let ffi_input: FfiInput = match serde_json::from_str(input_str) {
        Ok(v) => v,
        Err(e) => return error_json(&format!("Invalid JSON: {}", e)),
    };

    let name = ffi_input.name.as_deref().unwrap_or("headphone");
    let config_name = ffi_input.config.as_deref().unwrap_or("8_PEAKING_WITH_SHELVES");
    let params = match &ffi_input.params {
        Some(p) => p.to_process_params(),
        None => ProcessParams::default(),
    };
    let target_ref = ffi_input.target.as_deref();

    let result = match api::equalize_data(&ffi_input.frequency, &ffi_input.raw, target_ref, name, &params) {
        Ok(r) => r,
        Err(e) => return error_json(&format!("Processing error: {}", e)),
    };

    let peq_result = match api::optimize_parametric_eq(&ffi_input.frequency, &ffi_input.raw, name, &params, config_name, target_ref) {
        Ok(r) => Some(r),
        Err(_) => None,
    };

    let graphic_eq = match api::generate_graphic_eq_curve(&ffi_input.frequency, &ffi_input.raw, name, &params, target_ref) {
        Ok(s) => Some(s),
        Err(_) => None,
    };

    let output = FfiOutput {
        success: true,
        error: None,
        name: Some(result.name),
        frequency: Some(result.frequency),
        raw: Some(result.raw),
        smoothed: Some(result.smoothed),
        equalization: Some(result.equalization),
        target: Some(result.target),
        error_curve: Some(result.error),
        parametric_eq: peq_result,
        graphic_eq,
    };

    success_response(&output)
}

/// 获取库版本
///
/// # Safety
/// 返回的指针必须用 autoeq_free_string 释放
#[no_mangle]
pub unsafe extern "C" fn autoeq_version() -> *mut c_char {
    CString::new(env!("CARGO_PKG_VERSION")).map(|c| c.into_raw()).unwrap_or(ptr::null_mut())
}

/// 获取可用 PEQ 配置列表
///
/// # Safety
/// 返回的指针必须用 autoeq_free_string 释放
#[no_mangle]
pub unsafe extern "C" fn autoeq_configs() -> *mut c_char {
    let configs = api::get_available_configs();
    serde_json::to_string(&configs)
        .ok()
        .and_then(|j| CString::new(j).ok())
        .map(|c| c.into_raw())
        .unwrap_or(ptr::null_mut())
}

/// 预热：触发所有懒加载初始化 (LazyLock、FFT planner 等)
/// 建议在应用启动时调用一次，避免首次请求延迟
#[no_mangle]
pub extern "C" fn autoeq_warmup() {
    // 触发 PEQ_CONFIGS 懒加载
    let _ = crate::constants::PEQ_CONFIGS.len();

    // 用最小数据跑一次完整流程，触发 FFT planner 和所有静态初始化
    let freq = vec![20.0, 1000.0, 20000.0];
    let raw = vec![0.0, 0.0, 0.0];
    let target = vec![0.0, 0.0, 0.0];
    let params = crate::api::ProcessParams::default();
    let _ = crate::api::equalize_data(&freq, &raw, Some(&target), "warmup", &params);
    let _ = crate::api::optimize_parametric_eq(&freq, &raw, "warmup", &params, "8_PEAKING_WITH_SHELVES", Some(&target));
}

/// 释放字符串内存
///
/// # Safety
/// 指针必须由本库分配
#[no_mangle]
pub unsafe extern "C" fn autoeq_free_string(ptr: *mut c_char) {
    if !ptr.is_null() {
        unsafe { drop(CString::from_raw(ptr)); }
    }
}

// ===== 内部辅助函数 =====

fn success_response<T: serde::Serialize>(data: &T) -> *mut c_char {
    serde_json::to_string(data)
        .ok()
        .and_then(|j| CString::new(j).ok())
        .map(|c| c.into_raw())
        .unwrap_or(ptr::null_mut())
}

fn error_response(msg: &str) -> *mut c_char {
    let json = format!(r#"{{"error":"{}"}}"#, msg.replace('"', "\\\""));
    CString::new(json).ok().map(|c| c.into_raw()).unwrap_or(ptr::null_mut())
}

fn error_json(msg: &str) -> *mut c_char {
    let output = FfiOutput {
        success: false,
        error: Some(msg.to_string()),
        name: None, frequency: None, raw: None, smoothed: None,
        equalization: None, target: None, error_curve: None,
        parametric_eq: None, graphic_eq: None,
    };
    success_response(&output)
}
