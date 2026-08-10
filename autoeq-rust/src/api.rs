use crate::constants::*;
use crate::error::Result;
use crate::frequency_response::FrequencyResponse;
use crate::peq::{PEQ, PeqResult, FilterResult};
use std::path::Path;

/// Aggregated process parameters
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProcessParams {
    pub bass_boost_gain: f64,
    pub bass_boost_fc: f64,
    pub bass_boost_q: f64,
    pub treble_boost_gain: f64,
    pub treble_boost_fc: f64,
    pub treble_boost_q: f64,
    pub tilt: f64,
    pub fs: f64,
    pub max_gain: f64,
    pub preamp: f64,
    pub window_size: f64,
    pub treble_window_size: f64,
    pub treble_f_lower: f64,
    pub treble_f_upper: f64,
    pub treble_gain_k: f64,
    pub max_slope: f64,
    pub min_mean_error: bool,
}

impl Default for ProcessParams {
    fn default() -> Self {
        Self {
            bass_boost_gain: DEFAULT_BASS_BOOST_GAIN,
            bass_boost_fc: DEFAULT_BASS_BOOST_FC,
            bass_boost_q: DEFAULT_BASS_BOOST_Q,
            treble_boost_gain: DEFAULT_TREBLE_BOOST_GAIN,
            treble_boost_fc: DEFAULT_TREBLE_BOOST_FC,
            treble_boost_q: DEFAULT_TREBLE_BOOST_Q,
            tilt: DEFAULT_TILT,
            fs: DEFAULT_FS,
            max_gain: DEFAULT_MAX_GAIN,
            preamp: DEFAULT_PREAMP,
            window_size: DEFAULT_SMOOTHING_WINDOW_SIZE,
            treble_window_size: DEFAULT_TREBLE_SMOOTHING_WINDOW_SIZE,
            treble_f_lower: DEFAULT_TREBLE_SMOOTHING_F_LOWER,
            treble_f_upper: DEFAULT_TREBLE_SMOOTHING_F_UPPER,
            treble_gain_k: DEFAULT_TREBLE_GAIN_K,
            max_slope: DEFAULT_MAX_SLOPE,
            min_mean_error: false,
        }
    }
}

/// Result of equalize_data
#[derive(Debug, Clone, serde::Serialize)]
pub struct EqualizeResult {
    pub name: String,
    pub frequency: Vec<f64>,
    pub raw: Vec<f64>,
    pub smoothed: Vec<f64>,
    pub equalization: Vec<f64>,
    pub target: Vec<f64>,
    pub error: Vec<f64>,
}

/// Result of equalize_file
#[derive(Debug)]
pub struct FileEqualizeResult {
    pub eq_result: EqualizeResult,
    pub parametric_eq: PeqResult,
    pub graphic_eq_string: String,
}

/// Equalize frequency/raw data with optional target curve.
pub fn equalize_data(
    frequency: &[f64],
    raw: &[f64],
    target_curve: Option<&[f64]>,
    name: &str,
    params: &ProcessParams,
) -> Result<EqualizeResult> {
    let mut fr = FrequencyResponse::new(name, frequency.to_vec(), raw.to_vec())?;
    let _ = fr.interpolate(None, DEFAULT_STEP, DEFAULT_F_MIN, DEFAULT_F_MAX);
    let _ = fr.center(1000.0);

    // Build target
    let target = match target_curve {
        Some(tc) => {
            let mut t = FrequencyResponse::new("target", frequency.to_vec(), tc.to_vec())?;
            let _ = t.interpolate(Some(&fr.frequency), DEFAULT_STEP, DEFAULT_F_MIN, DEFAULT_F_MAX);
            let _ = t.center(1000.0);
            t
        }
        None => FrequencyResponse::new("flat_target", fr.frequency.clone(), vec![0.0; fr.frequency.len()])?,
    };

    fr.compensate(&target, params.bass_boost_gain, params.bass_boost_fc, params.bass_boost_q,
                  params.treble_boost_gain, params.treble_boost_fc, params.treble_boost_q,
                  params.tilt, params.fs, params.min_mean_error);
    fr.smoothen(params.window_size, params.treble_window_size,
               params.treble_f_lower, params.treble_f_upper);
    fr.equalize(params.max_gain, params.max_slope, 0.0, false,
               params.window_size, params.treble_window_size,
               params.treble_f_lower, params.treble_f_upper, params.treble_gain_k);

    Ok(EqualizeResult {
        name: fr.name.clone(),
        frequency: fr.frequency.clone(),
        raw: fr.raw.clone(),
        smoothed: fr.smoothed.clone(),
        equalization: fr.equalization.clone(),
        target: fr.target.clone(),
        error: fr.error.clone(),
    })
}

/// Full pipeline: equalize + parametric EQ + graphic EQ (single pass over the data).
pub fn equalize_data_full(
    frequency: &[f64],
    raw: &[f64],
    target_curve: Option<&[f64]>,
    name: &str,
    params: &ProcessParams,
    peq_config_name: &str,
) -> Result<(EqualizeResult, PeqResult, String)> {
    let eq_result = equalize_data(frequency, raw, target_curve, name, params)?;
    let peq_result = optimize_parametric_eq(frequency, raw, name, params, peq_config_name, target_curve)?;
    let graphic_eq = generate_graphic_eq_curve(frequency, raw, name, params, target_curve)?;
    Ok((eq_result, peq_result, graphic_eq))
}

/// Equalize a file and generate parametric EQ + graphic EQ.
pub fn equalize_file(
    input_path: &Path,
    target_path: Option<&Path>,
    name: Option<&str>,
    params: &ProcessParams,
    peq_config_name: &str,
) -> Result<FileEqualizeResult> {
    // Read input file (handle non-UTF-8 encodings)
    let csv_bytes = std::fs::read(input_path)?;
    let csv_text = String::from_utf8_lossy(&csv_bytes).to_string();
    let parsed = crate::csv::parse_csv(&csv_text)?;
    let fr_name = name.unwrap_or_else(|| {
        input_path.file_stem().and_then(|s| s.to_str()).unwrap_or("headphone")
    });

    let mut fr = FrequencyResponse::new(fr_name, parsed.frequency, parsed.raw)?;
    let _ = fr.interpolate(None, DEFAULT_STEP, DEFAULT_F_MIN, DEFAULT_F_MAX);
    let _ = fr.center(1000.0);

    // Build target
    let target = match target_path {
        Some(tp) => {
            let tc_bytes = std::fs::read(tp)?;
            let tc_text = String::from_utf8_lossy(&tc_bytes).to_string();
            let tc_parsed = crate::csv::parse_csv(&tc_text)?;
            let mut t = FrequencyResponse::new("target", tc_parsed.frequency, tc_parsed.raw)?;
            let _ = t.interpolate(Some(&fr.frequency), DEFAULT_STEP, DEFAULT_F_MIN, DEFAULT_F_MAX);
            let _ = t.center(1000.0);
            t
        }
        None => FrequencyResponse::new("flat_target", fr.frequency.clone(), vec![0.0; fr.frequency.len()])?,
    };

    fr.compensate(&target, params.bass_boost_gain, params.bass_boost_fc, params.bass_boost_q,
                  params.treble_boost_gain, params.treble_boost_fc, params.treble_boost_q,
                  params.tilt, params.fs, params.min_mean_error);
    fr.smoothen(params.window_size, params.treble_window_size,
               params.treble_f_lower, params.treble_f_upper);
    fr.equalize(params.max_gain, params.max_slope, 0.0, false,
               params.window_size, params.treble_window_size,
               params.treble_f_lower, params.treble_f_upper, params.treble_gain_k);

    // Optimize parametric EQ
    let config = PEQ_CONFIGS.get(peq_config_name)
        .or_else(|| PEQ_CONFIGS.get("8_PEAKING_WITH_SHELVES"))
        .unwrap();

    let eq_target = if !fr.equalization.is_empty() {
        fr.equalization.clone()
    } else {
        fr.error.clone()
    };

    let mut peq = PEQ::from_config(config, fr.frequency.clone(), params.fs, eq_target)?;
    peq.optimize(None)?;

    let preamp = if peq.filters.is_empty() {
        params.preamp
    } else {
        -peq.max_gain() - PREAMP_HEADROOM
    };

    let filters: Vec<FilterResult> = peq.filters.iter().map(|f| FilterResult {
        filter_type: f.filter_type(),
        fc: f.fc(),
        gain: f.gain(),
        q: f.q(),
    }).collect();

    let graphic_eq = fr.eqapo_graphic_eq(true, params.preamp, DEFAULT_GRAPHIC_EQ_STEP);

    Ok(FileEqualizeResult {
        eq_result: EqualizeResult {
            name: fr.name.clone(),
            frequency: fr.frequency.clone(),
            raw: fr.raw.clone(),
            smoothed: fr.smoothed.clone(),
            equalization: fr.equalization.clone(),
            target: fr.target.clone(),
            error: fr.error.clone(),
        },
        parametric_eq: PeqResult { preamp, filters },
        graphic_eq_string: graphic_eq,
    })
}

/// Optimize parametric EQ for given data.
pub fn optimize_parametric_eq(
    frequency: &[f64],
    raw: &[f64],
    name: &str,
    params: &ProcessParams,
    peq_config_name: &str,
    target_curve: Option<&[f64]>,
) -> Result<PeqResult> {
    optimize_parametric_eq_with_ranges(
        frequency, raw, name, params, peq_config_name, target_curve, None, None,
    )
}

/// optimize_parametric_eq with gain/q range overrides.
/// gain_range/q_range act as optimizer bound constraints (clamp), not post-hoc filters.
/// n_filters: AUTO 模式下自适应峰谷选频点的数量。
pub fn optimize_parametric_eq_with_ranges(
    frequency: &[f64],
    raw: &[f64],
    name: &str,
    params: &ProcessParams,
    peq_config_name: &str,
    target_curve: Option<&[f64]>,
    gain_range: Option<(f64, f64)>,
    q_range: Option<(f64, f64)>,
) -> Result<PeqResult> {
    optimize_parametric_eq_with_ranges_n(
        frequency, raw, name, params, peq_config_name, target_curve, gain_range, q_range, None,
    )
}

pub fn optimize_parametric_eq_with_ranges_n(
    frequency: &[f64],
    raw: &[f64],
    name: &str,
    params: &ProcessParams,
    peq_config_name: &str,
    target_curve: Option<&[f64]>,
    gain_range: Option<(f64, f64)>,
    q_range: Option<(f64, f64)>,
    n_filters: Option<usize>,
) -> Result<PeqResult> {
    let mut fr = FrequencyResponse::new(name, frequency.to_vec(), raw.to_vec())?;
    let _ = fr.interpolate(None, DEFAULT_STEP, DEFAULT_F_MIN, DEFAULT_F_MAX);
    let _ = fr.center(1000.0);

    let target = match target_curve {
        Some(tc) => {
            let mut t = FrequencyResponse::new("target", frequency.to_vec(), tc.to_vec())?;
            let _ = t.interpolate(Some(&fr.frequency), DEFAULT_STEP, DEFAULT_F_MIN, DEFAULT_F_MAX);
            let _ = t.center(1000.0);
            t
        }
        None => FrequencyResponse::new("flat_target", fr.frequency.clone(), vec![0.0; fr.frequency.len()])?,
    };

    fr.compensate(&target, params.bass_boost_gain, params.bass_boost_fc, params.bass_boost_q,
                  params.treble_boost_gain, params.treble_boost_fc, params.treble_boost_q,
                  params.tilt, params.fs, params.min_mean_error);
    fr.smoothen(params.window_size, params.treble_window_size,
               params.treble_f_lower, params.treble_f_upper);
    fr.equalize(params.max_gain, params.max_slope, 0.0, false,
               params.window_size, params.treble_window_size,
               params.treble_f_lower, params.treble_f_upper, params.treble_gain_k);

    let mut config = PEQ_CONFIGS.get(peq_config_name)
        .or_else(|| PEQ_CONFIGS.get("8_PEAKING_WITH_SHELVES"))
        .unwrap()
        .clone();

    // Apply gain/q range constraints to every filter's bounds (optimizer clamps within them)
    if gain_range.is_some() || q_range.is_some() {
        for fc in config.filters.iter_mut() {
            if let Some((lo, hi)) = gain_range {
                fc.min_gain = Some(lo);
                fc.max_gain = Some(hi);
            }
            if let Some((lo, hi)) = q_range {
                fc.min_q = Some(lo);
                fc.max_q = Some(hi);
            }
        }
    }

    let eq_target = if !fr.equalization.is_empty() {
        fr.equalization.clone()
    } else {
        fr.error.clone()
    };

    // AUTO：对数均匀频点（25Hz-18kHz，n 个），保证全频段覆盖且高频贴合。
    // （峰谷自适应会把低频峰谷全占满导致高频没滤波器，对数均匀最稳）
    if peq_config_name == "AUTO" {
        let n = n_filters.unwrap_or(8);
        if let Some(adaptive) = adaptive_peak_config(&eq_target, &fr.frequency, n) {
            config = adaptive;
            // 应用 gain/q 范围约束
            if gain_range.is_some() || q_range.is_some() {
                for fc in config.filters.iter_mut() {
                    if let Some((lo, hi)) = gain_range {
                        fc.min_gain = Some(lo);
                        fc.max_gain = Some(hi);
                    }
                    if let Some((lo, hi)) = q_range {
                        fc.min_q = Some(lo);
                        fc.max_q = Some(hi);
                    }
                }
            }
        }
    }

    let mut peq = PEQ::from_config(&config, fr.frequency.clone(), params.fs, eq_target)?;
    peq.optimize(None)?;

    let preamp = if peq.filters.is_empty() {
        params.preamp
    } else {
        -peq.max_gain() - PREAMP_HEADROOM
    };

    let filters: Vec<FilterResult> = peq.filters.iter().map(|f| FilterResult {
        filter_type: f.filter_type(),
        fc: f.fc(),
        gain: f.gain(),
        q: f.q(),
    }).collect();

    Ok(PeqResult { preamp, filters })
}

/// Generate EqualizerAPO graphic EQ curve string.
pub fn generate_graphic_eq_curve(
    frequency: &[f64],
    raw: &[f64],
    name: &str,
    params: &ProcessParams,
    target_curve: Option<&[f64]>,
) -> Result<String> {
    let mut fr = FrequencyResponse::new(name, frequency.to_vec(), raw.to_vec())?;
    let _ = fr.interpolate(None, DEFAULT_STEP, DEFAULT_F_MIN, DEFAULT_F_MAX);
    let _ = fr.center(1000.0);

    let target = match target_curve {
        Some(tc) => {
            let mut t = FrequencyResponse::new("target", frequency.to_vec(), tc.to_vec())?;
            let _ = t.interpolate(Some(&fr.frequency), DEFAULT_STEP, DEFAULT_F_MIN, DEFAULT_F_MAX);
            let _ = t.center(1000.0);
            t
        }
        None => FrequencyResponse::new("flat_target", fr.frequency.clone(), vec![0.0; fr.frequency.len()])?,
    };

    fr.compensate(&target, params.bass_boost_gain, params.bass_boost_fc, params.bass_boost_q,
                  params.treble_boost_gain, params.treble_boost_fc, params.treble_boost_q,
                  params.tilt, params.fs, params.min_mean_error);
    fr.smoothen(params.window_size, params.treble_window_size,
               params.treble_f_lower, params.treble_f_upper);
    fr.equalize(params.max_gain, params.max_slope, 0.0, false,
               params.window_size, params.treble_window_size,
               params.treble_f_lower, params.treble_f_upper, params.treble_gain_k);

    Ok(fr.eqapo_graphic_eq(true, params.preamp, DEFAULT_GRAPHIC_EQ_STEP))
}

/// Get list of available PEQ config names.
pub fn get_available_configs() -> Vec<&'static str> {
    PEQ_CONFIGS.keys().copied().collect()
}

/// 对数均匀频点（25Hz-18kHz，n 个），fc 固定只优化 gain/q——保证全频段覆盖且高频贴合。
/// （峰谷自适应会把低频峰谷全占满导致高频没滤波器，对数均匀最稳）
pub fn adaptive_peak_config(
    _target: &[f64],
    _frequency: &[f64],
    n_filters: usize,
) -> Option<PeqConfig> {
    use crate::constants::FilterConfig;
    use crate::constants::FilterType;

    let n = if n_filters < 2 { 2 } else { n_filters };
    let filters: Vec<FilterConfig> = (0..n).map(|i| {
        let fc = 25.0_f64 * (18000.0_f64 / 25.0_f64).powf(i as f64 / (n - 1) as f64);
        FilterConfig {
            filter_type: Some(FilterType::Peaking),
            fc: Some(fc),
            q: None,
            gain: None,
            min_fc: None,
            max_fc: None,
            min_q: None,
            max_q: None,
            min_gain: None,
            max_gain: None,
        }
    }).collect();

    Some(PeqConfig {
        optimizer: crate::constants::OptimizerConfig { max_time: Some(0.5), ..Default::default() },
        filter_defaults: None,
        filters,
    })
}
