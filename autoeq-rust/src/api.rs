use crate::constants::*;
use crate::error::Result;
use crate::frequency_response::FrequencyResponse;
use crate::peq::{PEQ, PeqResult, FilterResult};
use std::path::Path;

/// Aggregated process parameters
#[derive(Debug, Clone)]
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
        }
    }
}

/// Result of equalize_data
#[derive(Debug, Clone)]
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
                  params.tilt, params.fs);
    fr.smoothen(DEFAULT_SMOOTHING_WINDOW_SIZE, DEFAULT_TREBLE_SMOOTHING_WINDOW_SIZE,
               DEFAULT_TREBLE_SMOOTHING_F_LOWER, DEFAULT_TREBLE_SMOOTHING_F_UPPER);
    fr.equalize(params.max_gain, DEFAULT_MAX_SLOPE, 0.0, false,
               DEFAULT_SMOOTHING_WINDOW_SIZE, DEFAULT_TREBLE_SMOOTHING_WINDOW_SIZE,
               DEFAULT_TREBLE_SMOOTHING_F_LOWER, DEFAULT_TREBLE_SMOOTHING_F_UPPER, DEFAULT_TREBLE_GAIN_K);

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
                  params.tilt, params.fs);
    fr.smoothen(DEFAULT_SMOOTHING_WINDOW_SIZE, DEFAULT_TREBLE_SMOOTHING_WINDOW_SIZE,
               DEFAULT_TREBLE_SMOOTHING_F_LOWER, DEFAULT_TREBLE_SMOOTHING_F_UPPER);
    fr.equalize(params.max_gain, DEFAULT_MAX_SLOPE, 0.0, false,
               DEFAULT_SMOOTHING_WINDOW_SIZE, DEFAULT_TREBLE_SMOOTHING_WINDOW_SIZE,
               DEFAULT_TREBLE_SMOOTHING_F_LOWER, DEFAULT_TREBLE_SMOOTHING_F_UPPER, DEFAULT_TREBLE_GAIN_K);

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
        -peq.max_gain()
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
                  params.tilt, params.fs);
    fr.smoothen(DEFAULT_SMOOTHING_WINDOW_SIZE, DEFAULT_TREBLE_SMOOTHING_WINDOW_SIZE,
               DEFAULT_TREBLE_SMOOTHING_F_LOWER, DEFAULT_TREBLE_SMOOTHING_F_UPPER);
    fr.equalize(params.max_gain, DEFAULT_MAX_SLOPE, 0.0, false,
               DEFAULT_SMOOTHING_WINDOW_SIZE, DEFAULT_TREBLE_SMOOTHING_WINDOW_SIZE,
               DEFAULT_TREBLE_SMOOTHING_F_LOWER, DEFAULT_TREBLE_SMOOTHING_F_UPPER, DEFAULT_TREBLE_GAIN_K);

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
                  params.tilt, params.fs);
    fr.smoothen(DEFAULT_SMOOTHING_WINDOW_SIZE, DEFAULT_TREBLE_SMOOTHING_WINDOW_SIZE,
               DEFAULT_TREBLE_SMOOTHING_F_LOWER, DEFAULT_TREBLE_SMOOTHING_F_UPPER);
    fr.equalize(params.max_gain, DEFAULT_MAX_SLOPE, 0.0, false,
               DEFAULT_SMOOTHING_WINDOW_SIZE, DEFAULT_TREBLE_SMOOTHING_WINDOW_SIZE,
               DEFAULT_TREBLE_SMOOTHING_F_LOWER, DEFAULT_TREBLE_SMOOTHING_F_UPPER, DEFAULT_TREBLE_GAIN_K);

    Ok(fr.eqapo_graphic_eq(true, params.preamp, DEFAULT_GRAPHIC_EQ_STEP))
}

/// Get list of available PEQ config names.
pub fn get_available_configs() -> Vec<&'static str> {
    PEQ_CONFIGS.keys().copied().collect()
}
