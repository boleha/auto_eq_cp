use crate::constants::*;
use crate::dsp;
use crate::error::{AutoEqError, Result};
use crate::utils;
use std::collections::BTreeMap;

/// Core frequency response data structure.
/// All columns share the same frequency axis. Empty vec means no data.
#[derive(Debug, Clone)]
pub struct FrequencyResponse {
    pub name: String,
    pub frequency: Vec<f64>,
    pub raw: Vec<f64>,
    pub smoothed: Vec<f64>,
    pub error: Vec<f64>,
    pub error_smoothed: Vec<f64>,
    pub equalization: Vec<f64>,
    pub parametric_eq: Vec<f64>,
    pub fixed_band_eq: Vec<f64>,
    pub equalized_raw: Vec<f64>,
    pub equalized_smoothed: Vec<f64>,
    pub target: Vec<f64>,
}

/// Result of the equalize operation
pub struct EqualizeResult {
    pub equalization: Vec<f64>,
    pub smoothed_error: Vec<f64>,
    pub limited_ltr: Vec<f64>,
    pub clipped_ltr: Vec<bool>,
    pub limited_rtl: Vec<f64>,
    pub clipped_rtl: Vec<bool>,
    pub peak_inds: Vec<usize>,
    pub dip_inds: Vec<usize>,
    pub rtl_start: usize,
    pub limit_free_mask: Vec<bool>,
}

impl FrequencyResponse {
    /// Create a new FrequencyResponse from frequency and raw data.
    pub fn new(name: &str, frequency: Vec<f64>, raw: Vec<f64>) -> Result<Self> {
        let name = name.trim().to_string();
        if name.is_empty() {
            return Err(AutoEqError::InvalidInput("name must not be empty".into()));
        }

        let mut fr = Self {
            name,
            frequency,
            raw,
            smoothed: Vec::new(),
            error: Vec::new(),
            error_smoothed: Vec::new(),
            equalization: Vec::new(),
            parametric_eq: Vec::new(),
            fixed_band_eq: Vec::new(),
            equalized_raw: Vec::new(),
            equalized_smoothed: Vec::new(),
            target: Vec::new(),
        };

        // If no frequency data, generate default grid
        if fr.frequency.is_empty() {
            fr.frequency = Self::default_frequencies();
            fr.raw = vec![0.0; fr.frequency.len()];
        }

        // Check for duplicate frequencies
        fr.check_duplicates()?;
        // Sort by frequency
        fr.sort_by_frequency();

        Ok(fr)
    }

    /// Create from parsed CSV data
    pub fn from_csv(name: &str, frequency: Vec<f64>, raw: Vec<f64>) -> Result<Self> {
        Self::new(name, frequency, raw)
    }

    /// Default frequency grid: 20-20000 Hz, step 1.01
    pub fn default_frequencies() -> Vec<f64> {
        utils::generate_frequencies(DEFAULT_F_MIN, DEFAULT_F_MAX, DEFAULT_STEP)
    }

    fn check_duplicates(&self) -> Result<()> {
        for i in 1..self.frequency.len() {
            if (self.frequency[i] - self.frequency[i - 1]).abs() < 1e-10 {
                return Err(AutoEqError::InvalidInput(format!(
                    "Duplicate frequency: {}", self.frequency[i]
                )));
            }
        }
        Ok(())
    }

    fn sort_by_frequency(&mut self) {
        let n = self.frequency.len();
        let mut indices: Vec<usize> = (0..n).collect();
        indices.sort_by(|&a, &b| self.frequency[a].partial_cmp(&self.frequency[b]).unwrap());

        let sorted_freq: Vec<f64> = indices.iter().map(|&i| self.frequency[i]).collect();

        let reorder = |v: &mut Vec<f64>, indices: &[usize], n: usize| {
            if v.len() == n {
                let saved: Vec<f64> = indices.iter().map(|&i| v[i]).collect();
                v.copy_from_slice(&saved);
            }
        };

        reorder(&mut self.raw, &indices, n);
        self.frequency = sorted_freq;
        reorder(&mut self.smoothed, &indices, n);
        reorder(&mut self.error, &indices, n);
        reorder(&mut self.error_smoothed, &indices, n);
        reorder(&mut self.equalization, &indices, n);
        reorder(&mut self.parametric_eq, &indices, n);
        reorder(&mut self.fixed_band_eq, &indices, n);
        reorder(&mut self.equalized_raw, &indices, n);
        reorder(&mut self.equalized_smoothed, &indices, n);
        reorder(&mut self.target, &indices, n);
    }

    /// Linear interpolation in log10(frequency) space to a new frequency grid.
    pub fn interpolate(&mut self, f: Option<&[f64]>, f_step: f64, f_min: f64, f_max: f64) -> Result<()> {
        // Filter out NaN from raw
        let mut valid_freq = Vec::new();
        let mut valid_raw = Vec::new();
        for i in 0..self.frequency.len() {
            if i < self.raw.len() && !self.raw[i].is_nan() {
                valid_freq.push(self.frequency[i]);
                valid_raw.push(self.raw[i]);
            }
        }

        if valid_freq.is_empty() {
            return Err(AutoEqError::InvalidInput("No valid data to interpolate".into()));
        }

        let target_f = match f {
            Some(f) => f.to_vec(),
            None => utils::generate_frequencies(f_min, f_max, f_step),
        };

        // Interpolate in log10 frequency space
        let log_freq: Vec<f64> = valid_freq.iter().map(|f| f.max(0.001).log10()).collect();
        let new_log_freq: Vec<f64> = target_f.iter().map(|f| f.max(0.001).log10()).collect();

        self.raw = lerp_interpolate(&log_freq, &valid_raw, &new_log_freq);

        // Interpolate other non-empty columns
        let interpolate_col = |col: &Vec<f64>, log_freq: &[f64], new_log_freq: &[f64]| -> Vec<f64> {
            if col.is_empty() || col.len() != valid_freq.len() {
                return Vec::new();
            }
            // Filter NaN from this column
            let mut valid_data = Vec::new();
            let mut valid_log = Vec::new();
            for i in 0..col.len() {
                if !col[i].is_nan() && i < log_freq.len() {
                    valid_data.push(col[i]);
                    valid_log.push(log_freq[i]);
                }
            }
            if valid_data.is_empty() { return Vec::new(); }
            lerp_interpolate(&valid_log, &valid_data, new_log_freq)
        };

        self.smoothed = interpolate_col(&self.smoothed, &log_freq, &new_log_freq);
        self.error = interpolate_col(&self.error, &log_freq, &new_log_freq);
        self.error_smoothed = interpolate_col(&self.error_smoothed, &log_freq, &new_log_freq);
        self.equalization = interpolate_col(&self.equalization, &log_freq, &new_log_freq);
        self.equalized_raw = interpolate_col(&self.equalized_raw, &log_freq, &new_log_freq);
        self.equalized_smoothed = interpolate_col(&self.equalized_smoothed, &log_freq, &new_log_freq);
        self.target = interpolate_col(&self.target, &log_freq, &new_log_freq);

        self.frequency = target_f;
        self.parametric_eq = Vec::new();
        self.fixed_band_eq = Vec::new();

        Ok(())
    }

    /// Normalize so gain at the given frequency is 0 dB.
    /// Returns the negative of the gain at that frequency.
    pub fn center(&mut self, frequency: f64) -> f64 {
        if self.frequency.is_empty() || self.raw.is_empty() {
            return 0.0;
        }

        // Find gain at the center frequency via log interpolation
        let log_f: Vec<f64> = self.frequency.iter().map(|f| f.max(0.001).log10()).collect();
        let log_target = frequency.max(0.001).log10();
        let diff = lerp_single(&log_f, &self.raw, log_target);

        // Subtract from raw
        for v in self.raw.iter_mut() { *v -= diff; }
        for v in self.smoothed.iter_mut() { if !v.is_nan() { *v -= diff; } }
        // Add to error (error = raw - target, so centering shifts error in opposite direction)
        for v in self.error.iter_mut() { if !v.is_nan() { *v += diff; } }
        for v in self.error_smoothed.iter_mut() { if !v.is_nan() { *v += diff; } }

        -diff
    }

    /// Compute error = raw - target, with bass/treble boost shelves and tilt.
    pub fn compensate(
        &mut self,
        target: &FrequencyResponse,
        bass_boost_gain: f64,
        bass_boost_fc: f64,
        bass_boost_q: f64,
        treble_boost_gain: f64,
        treble_boost_fc: f64,
        treble_boost_q: f64,
        tilt: f64,
        fs: f64,
    ) {
        // Prepare target: interpolate and center
        let mut target_fr = target.clone();
        let _ = target_fr.interpolate(Some(&self.frequency), DEFAULT_STEP, DEFAULT_F_MIN, DEFAULT_F_MAX);
        let _ = target_fr.center(1000.0);

        // Create target: target.raw + bass_shelf + treble_shelf + tilt
        let bass_shelf = biquad_response_low_shelf(&self.frequency, fs, bass_boost_fc, bass_boost_q, bass_boost_gain);
        let treble_shelf = biquad_response_high_shelf(&self.frequency, fs, treble_boost_fc, treble_boost_q, treble_boost_gain);
        let tilt_curve = utils::log_tilt(&self.frequency, tilt);

        self.target = Vec::with_capacity(self.frequency.len());
        for i in 0..self.frequency.len() {
            let t = if i < target_fr.raw.len() { target_fr.raw[i] } else { 0.0 };
            let b = if i < bass_shelf.len() { bass_shelf[i] } else { 0.0 };
            let tr = if i < treble_shelf.len() { treble_shelf[i] } else { 0.0 };
            let ti = if i < tilt_curve.len() { tilt_curve[i] } else { 0.0 };
            self.target.push(t + b + tr + ti);
        }

        // Compute error
        self.error = self.raw.iter().zip(self.target.iter()).map(|(r, t)| r - t).collect();
    }

    /// Apply Savitzky-Golay smoothing with sigmoid blending between normal and treble windows.
    pub fn smoothen(
        &mut self,
        window_size: f64,
        treble_window_size: f64,
        treble_f_lower: f64,
        treble_f_upper: f64,
    ) {
        if !self.raw.is_empty() {
            self.smoothed = smoothen_data(&self.frequency, &self.raw, window_size, treble_window_size, treble_f_lower, treble_f_upper);
        }
        if !self.error.is_empty() {
            self.error_smoothed = smoothen_data(&self.frequency, &self.error, window_size, treble_window_size, treble_f_lower, treble_f_upper);
        }
    }

    /// Compute equalization curve using bidirectional slope limiting.
    pub fn equalize(
        &mut self,
        max_gain: f64,
        max_slope: f64,
        max_slope_decay: f64,
        concha_interference: bool,
        window_size: f64,
        treble_window_size: f64,
        treble_f_lower: f64,
        treble_f_upper: f64,
        treble_gain_k: f64,
    ) -> EqualizeResult {
        // Smooth the error
        let smoothed_error = if !self.error.is_empty() {
            smoothen_data(&self.frequency, &self.error, window_size, treble_window_size, treble_f_lower, treble_f_upper)
        } else {
            vec![0.0; self.frequency.len()]
        };

        let x = &self.frequency;
        let y: Vec<f64> = smoothed_error.iter().map(|v| -v).collect();

        // Find peaks and dips
        let (peak_inds, _proms, _widths, _heights) = dsp::find_peaks_with_props(&y, 1.0);
        let dip_inds = dsp::find_dips(&y);

        // If no peaks and no dips, equalization is just y
        if peak_inds.is_empty() && dip_inds.is_empty() {
            self.equalization = y.clone();
            self.equalized_raw = self.raw.iter().zip(y.iter()).map(|(r, e)| r + e).collect();
            if !self.smoothed.is_empty() {
                self.equalized_smoothed = self.smoothed.iter().zip(y.iter()).map(|(s, e)| s + e).collect();
            }
            return EqualizeResult {
                equalization: y,
                smoothed_error,
                limited_ltr: vec![],
                clipped_ltr: vec![],
                limited_rtl: vec![],
                clipped_rtl: vec![],
                peak_inds,
                dip_inds,
                rtl_start: 0,
                limit_free_mask: vec![],
            };
        }

        // Protection mask
        let limit_free_mask = protection_mask(&y, &peak_inds, &dip_inds);

        // Find RTL start
        let rtl_start = find_rtl_start(&y, &peak_inds, &dip_inds);

        // Bidirectional slope limiting
        let (limited_ltr, clipped_ltr, _regions_ltr) = limited_ltr_slope(
            x, &y, max_slope, max_slope_decay, 0, &peak_inds, &limit_free_mask, concha_interference,
        );
        let (limited_rtl, clipped_rtl, _regions_rtl) = limited_rtl_slope(
            x, &y, max_slope, max_slope_decay, rtl_start, &peak_inds, &limit_free_mask, concha_interference,
        );

        // Combine with min
        let mut combined: Vec<f64> = limited_ltr.iter().zip(limited_rtl.iter()).map(|(a, b)| a.min(*b)).collect();

        // Apply treble gain coefficient
        let gain_k = utils::log_f_sigmoid(&self.frequency, treble_f_lower, treble_f_upper, 1.0, treble_gain_k);
        for i in 0..combined.len() {
            combined[i] *= gain_k[i];
        }

        // Clip to max gain
        for v in combined.iter_mut() {
            if *v > max_gain { *v = max_gain; }
        }

        // Final smoothing
        let final_eq = smoothen_data(&self.frequency, &combined, 1.0 / 5.0, 1.0 / 5.0, treble_f_lower, treble_f_upper);

        self.equalization = final_eq.clone();
        self.equalized_raw = self.raw.iter().zip(final_eq.iter()).map(|(r, e)| r + e).collect();
        if !self.smoothed.is_empty() {
            self.equalized_smoothed = self.smoothed.iter().zip(final_eq.iter()).map(|(s, e)| s + e).collect();
        }

        EqualizeResult {
            equalization: final_eq,
            smoothed_error,
            limited_ltr,
            clipped_ltr,
            limited_rtl,
            clipped_rtl,
            peak_inds,
            dip_inds,
            rtl_start,
            limit_free_mask,
        }
    }

    /// Full processing pipeline: interpolate -> center -> compensate -> smoothen -> equalize
    pub fn process(
        &mut self,
        target: &FrequencyResponse,
        bass_boost_gain: f64,
        bass_boost_fc: f64,
        bass_boost_q: f64,
        treble_boost_gain: f64,
        treble_boost_fc: f64,
        treble_boost_q: f64,
        tilt: f64,
        fs: f64,
        max_gain: f64,
        max_slope: f64,
        concha_interference: bool,
        window_size: f64,
        treble_window_size: f64,
        treble_f_lower: f64,
        treble_f_upper: f64,
        treble_gain_k: f64,
    ) -> EqualizeResult {
        let _ = self.interpolate(None, DEFAULT_STEP, DEFAULT_F_MIN, DEFAULT_F_MAX);
        let _ = self.center(1000.0);
        self.compensate(target, bass_boost_gain, bass_boost_fc, bass_boost_q,
                       treble_boost_gain, treble_boost_fc, treble_boost_q, tilt, fs);
        self.smoothen(window_size, treble_window_size, treble_f_lower, treble_f_upper);
        self.equalize(max_gain, max_slope, 0.0, concha_interference,
                     window_size, treble_window_size, treble_f_lower, treble_f_upper, treble_gain_k)
    }

    /// Generate EqualizerAPO GraphicEQ format string
    pub fn eqapo_graphic_eq(&self, normalize: bool, preamp: f64, f_step: f64) -> String {
        let graphic_f = utils::generate_frequencies(DEFAULT_F_MIN, DEFAULT_F_MAX, f_step);

        // Interpolate equalization to graphic EQ frequency grid
        let log_freq: Vec<f64> = self.frequency.iter().map(|f| f.max(0.001).log10()).collect();
        let log_graphic: Vec<f64> = graphic_f.iter().map(|f| f.max(0.001).log10()).collect();

        let eq_data = if !self.equalization.is_empty() {
            &self.equalization
        } else if !self.error.is_empty() {
            &self.error
        } else {
            &self.raw
        };

        let mut graphic_eq = lerp_interpolate(&log_freq, eq_data, &log_graphic);

        // Prevent bass boost below lowest frequency
        if !graphic_eq.is_empty() && graphic_eq[0] > 0.0 {
            graphic_eq[0] = 0.0;
        }

        // Normalize
        if normalize {
            let max_val = graphic_eq.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            if max_val.is_finite() {
                let norm = -max_val - PREAMP_HEADROOM + preamp;
                for v in graphic_eq.iter_mut() { *v += norm; }
            }
        }

        // Format as EqualizerAPO GraphicEQ string
        let parts: Vec<String> = graphic_f.iter().zip(graphic_eq.iter()).map(|(f, g)| {
            format!("{}{:.1}", format_frequency(*f), g)
        }).collect();

        format!("GraphicEQ: {}", parts.join("; "))
    }

    /// Convert to dictionary (for serialization)
    pub fn to_dict(&self) -> BTreeMap<String, Vec<f64>> {
        let mut map = BTreeMap::new();
        map.insert("frequency".to_string(), self.frequency.clone());
        if !self.raw.is_empty() { map.insert("raw".to_string(), self.raw.clone()); }
        if !self.smoothed.is_empty() { map.insert("smoothed".to_string(), self.smoothed.clone()); }
        if !self.error.is_empty() { map.insert("error".to_string(), self.error.clone()); }
        if !self.error_smoothed.is_empty() { map.insert("error_smoothed".to_string(), self.error_smoothed.clone()); }
        if !self.equalization.is_empty() { map.insert("equalization".to_string(), self.equalization.clone()); }
        if !self.target.is_empty() { map.insert("target".to_string(), self.target.clone()); }
        if !self.equalized_raw.is_empty() { map.insert("equalized_raw".to_string(), self.equalized_raw.clone()); }
        map
    }
}

// --- Helper functions ---

/// Linear interpolation
fn lerp_interpolate(xp: &[f64], fp: &[f64], x_new: &[f64]) -> Vec<f64> {
    if xp.is_empty() || fp.is_empty() { return vec![0.0; x_new.len()]; }

    x_new.iter().map(|&x| {
        if x <= xp[0] { return fp[0]; }
        if x >= xp[xp.len() - 1] { return fp[fp.len() - 1]; }

        // Binary search
        let mut lo = 0;
        let mut hi = xp.len() - 1;
        while lo < hi - 1 {
            let mid = (lo + hi) / 2;
            if xp[mid] <= x { lo = mid; } else { hi = mid; }
        }

        let t = (x - xp[lo]) / (xp[hi] - xp[lo]);
        fp[lo] + t * (fp[hi] - fp[lo])
    }).collect()
}

/// Single-point linear interpolation
fn lerp_single(xp: &[f64], fp: &[f64], x: f64) -> f64 {
    if xp.is_empty() || fp.is_empty() { return 0.0; }
    if x <= xp[0] { return fp[0]; }
    if x >= xp[xp.len() - 1] { return fp[fp.len() - 1]; }

    let mut lo = 0;
    let mut hi = xp.len() - 1;
    while lo < hi - 1 {
        let mid = (lo + hi) / 2;
        if xp[mid] <= x { lo = mid; } else { hi = mid; }
    }

    let t = (x - xp[lo]) / (xp[hi] - xp[lo]);
    fp[lo] + t * (fp[hi] - fp[lo])
}

/// Smoothing with two-pass Savitzky-Golay and sigmoid blending
fn smoothen_data(
    f: &[f64], data: &[f64],
    window_size: f64, treble_window_size: f64,
    treble_f_lower: f64, treble_f_upper: f64,
) -> Vec<f64> {
    if data.is_empty() { return Vec::new(); }

    let n_normal = utils::smoothing_window_size(f, window_size);
    let n_treble = utils::smoothing_window_size(f, treble_window_size);

    let y_normal = dsp::savgol_filter(data, n_normal.max(3), 2);
    let y_treble = dsp::savgol_filter(data, n_treble.max(3), 2);

    // Sigmoid blending
    let k_treble = utils::log_f_sigmoid(f, treble_f_lower, treble_f_upper, 0.0, 1.0);

    y_normal.iter().zip(y_treble.iter()).zip(k_treble.iter()).map(|((yn, yt), kt)| {
        yn * (1.0 - kt) + yt * kt
    }).collect()
}

/// Biquad LowShelf frequency response
fn biquad_response_low_shelf(f: &[f64], fs: f64, fc: f64, q: f64, gain: f64) -> Vec<f64> {
    let a = 10.0_f64.powf(gain / 40.0);
    let w0 = 2.0 * std::f64::consts::PI * fc / fs;
    let alpha = w0.sin() / (2.0 * q);
    let cos_w0 = w0.cos();
    let sqrt_a = a.sqrt();

    let a0 = (a + 1.0) + (a - 1.0) * cos_w0 + 2.0 * sqrt_a * alpha;
    // a1 and a2 as returned by biquad_coefficients (already negated)
    let a1 = -(-2.0 * ((a - 1.0) + (a + 1.0) * cos_w0)) / a0;
    let a2 = -((a + 1.0) + (a - 1.0) * cos_w0 - 2.0 * sqrt_a * alpha) / a0;
    let b0 = (a * ((a + 1.0) - (a - 1.0) * cos_w0 + 2.0 * sqrt_a * alpha)) / a0;
    let b1 = (2.0 * a * ((a - 1.0) - (a + 1.0) * cos_w0)) / a0;
    let b2 = (a * ((a + 1.0) - (a - 1.0) * cos_w0 - 2.0 * sqrt_a * alpha)) / a0;

    // Python fr property negates a1 and a2 again: use -a1, -a2
    let a1n = -a1;
    let a2n = -a2;

    f.iter().map(|&fi| {
        let w = 2.0 * std::f64::consts::PI * fi / fs;
        let phi = 4.0 * (w / 2.0).sin().powi(2);
        let num = (b0 + b1 + b2).powi(2) + (b0 * b2 * phi - (b1 * (b0 + b2) + 4.0 * b0 * b2)) * phi;
        let den = (1.0 + a1n + a2n).powi(2) + (a2n * phi - (a1n * (1.0 + a2n) + 4.0 * a2n)) * phi;
        10.0 * num.max(1e-30).log10() - 10.0 * den.max(1e-30).log10()
    }).collect()
}

/// Biquad HighShelf frequency response
fn biquad_response_high_shelf(f: &[f64], fs: f64, fc: f64, q: f64, gain: f64) -> Vec<f64> {
    let a = 10.0_f64.powf(gain / 40.0);
    let w0 = 2.0 * std::f64::consts::PI * fc / fs;
    let alpha = w0.sin() / (2.0 * q);
    let cos_w0 = w0.cos();
    let sqrt_a = a.sqrt();

    let a0 = (a + 1.0) - (a - 1.0) * cos_w0 + 2.0 * sqrt_a * alpha;
    let a1 = -(2.0 * ((a - 1.0) - (a + 1.0) * cos_w0)) / a0;
    let a2 = -((a + 1.0) - (a - 1.0) * cos_w0 - 2.0 * sqrt_a * alpha) / a0;
    let b0 = (a * ((a + 1.0) + (a - 1.0) * cos_w0 + 2.0 * sqrt_a * alpha)) / a0;
    let b1 = (-2.0 * a * ((a - 1.0) + (a + 1.0) * cos_w0)) / a0;
    let b2 = (a * ((a + 1.0) + (a - 1.0) * cos_w0 - 2.0 * sqrt_a * alpha)) / a0;

    // Python fr property negates a1 and a2 again: use -a1, -a2
    let a1n = -a1;
    let a2n = -a2;

    f.iter().map(|&fi| {
        let w = 2.0 * std::f64::consts::PI * fi / fs;
        let phi = 4.0 * (w / 2.0).sin().powi(2);
        let num = (b0 + b1 + b2).powi(2) + (b0 * b2 * phi - (b1 * (b0 + b2) + 4.0 * b0 * b2)) * phi;
        let den = (1.0 + a1n + a2n).powi(2) + (a2n * phi - (a1n * (1.0 + a2n) + 4.0 * a2n)) * phi;
        10.0 * num.max(1e-30).log10() - 10.0 * den.max(1e-30).log10()
    }).collect()
}

/// Compute protection mask around deep dips
fn protection_mask(y: &[f64], peak_inds: &[usize], dip_inds: &[usize]) -> Vec<bool> {
    let mut mask = vec![false; y.len()];

    if dip_inds.len() < 3 { return mask; }

    let mut extended_dips = dip_inds.to_vec();
    // If last peak is after last dip, find a dip after the last peak
    if let (Some(&last_peak), Some(&last_dip)) = (peak_inds.last(), dip_inds.last()) {
        if last_peak > last_dip {
            // Find minimum after last peak
            let mut min_val = f64::INFINITY;
            let mut min_idx = y.len() - 1;
            for i in last_peak..y.len() {
                if y[i] < min_val {
                    min_val = y[i];
                    min_idx = i;
                }
            }
            extended_dips.push(min_idx);
        }
    }

    if extended_dips.len() < 3 { return mask; }

    // For each interior dip, protect the region around it
    for i in 1..(extended_dips.len() - 1) {
        let dip = extended_dips[i];
        let target_left = y[extended_dips[i - 1]];
        let target_right = y[extended_dips[i + 1]];

        // Find left boundary: rightmost index before dip where y >= target_left
        let mut left_ind = 0;
        for j in (0..dip).rev() {
            if y[j] >= target_left {
                left_ind = j + 1;
                break;
            }
        }

        // Find right boundary: leftmost index after dip where y >= target_right
        let mut right_ind = y.len() - 1;
        for j in (dip + 1)..y.len() {
            if y[j] >= target_right {
                right_ind = j - 1;
                break;
            }
        }

        for j in left_ind..=right_ind {
            mask[j] = true;
        }
    }

    mask
}

/// Find RTL traversal start index
fn find_rtl_start(y: &[f64], peak_inds: &[usize], dip_inds: &[usize]) -> usize {
    if y.is_empty() { return 0; }

    let last_peak = peak_inds.last().copied();
    let last_dip = dip_inds.last().copied();

    match (last_peak, last_dip) {
        (Some(lp), Some(ld)) if lp > ld => {
            // Last peak is after last dip
            let threshold = if !dip_inds.is_empty() {
                y[dip_inds[dip_inds.len() - 1]]
            } else {
                y[0].max(y[y.len() - 1])
            };
            // Find first index after last peak where y <= threshold
            for i in (lp + 1)..y.len() {
                if y[i] <= threshold { return i; }
            }
            y.len() - 1
        }
        (Some(_), Some(ld)) => ld,
        (Some(lp), None) => lp,
        (None, Some(ld)) => ld,
        (None, None) => y.len() - 1,
    }
}

/// Left-to-right slope limiting with peak-based region reversion
fn limited_ltr_slope(
    x: &[f64],
    y: &[f64],
    max_slope: f64,
    max_slope_decay: f64,
    start_index: usize,
    peak_inds: &[usize],
    limit_free_mask: &[bool],
    concha_interference: bool,
) -> (Vec<f64>, Vec<bool>, Vec<[usize; 2]>) {
    let n = x.len();
    let mut limited = vec![0.0; n];
    let mut clipped = vec![false; n];
    let mut regions: Vec<[usize; 2]> = Vec::new();

    for i in 0..n {
        if i <= start_index {
            limited[i] = y[i];
            clipped[i] = false;
            continue;
        }

        let slope = utils::log_log_gradient(x[i - 1], x[i], limited[i - 1], y[i]);
        let mut local_limit = max_slope;

        if concha_interference && x[i] >= 8000.0 && x[i] <= 11500.0 {
            local_limit = max_slope / 4.0;
        }

        // Decay if in a clipped region
        if clipped[i - 1] && !regions.is_empty() && max_slope_decay > 0.0 {
            let region_start = regions.last().unwrap()[0];
            let octaves_from_start = (x[i] / x[region_start]).log2();
            local_limit *= (1.0 - max_slope_decay).powf(octaves_from_start);
        }

        let in_limit_free = i < limit_free_mask.len() && limit_free_mask[i];

        if slope > local_limit && !in_limit_free {
            // Clip
            if !clipped[i - 1] {
                regions.push([i, 0]);
            }
            clipped[i] = true;
            let octaves = (x[i] / x[i - 1]).log2();
            limited[i] = limited[i - 1] + local_limit * octaves;
        } else {
            limited[i] = y[i];
            if clipped[i - 1] && !regions.is_empty() {
                // End of clipped region
                let region_start = regions.last().unwrap()[0];
                regions.last_mut().unwrap()[1] = i;

                // Check if any peak falls in [region_start, i)
                let has_peak = peak_inds.iter().any(|&p| p >= region_start && p < i);
                if !has_peak {
                    // Revert: no peak in this region
                    for j in region_start..i {
                        limited[j] = y[j];
                        clipped[j] = false;
                    }
                    regions.pop();
                }
            }
            clipped[i] = false;
        }
    }

    // Close last open region (if end is still 0, it means it wasn't closed)
    if let Some(last) = regions.last_mut() {
        if last[1] == 0 {
            last[1] = n - 1;
        }
    }

    (limited, clipped, regions)
}

/// Right-to-left slope limiting (flips, calls LTR, flips back)
fn limited_rtl_slope(
    x: &[f64],
    y: &[f64],
    max_slope: f64,
    max_slope_decay: f64,
    start_index: usize,
    peak_inds: &[usize],
    limit_free_mask: &[bool],
    concha_interference: bool,
) -> (Vec<f64>, Vec<bool>, Vec<[usize; 2]>) {
    let n = x.len();

    // Flip all arrays
    let x_flip: Vec<f64> = x.iter().copied().rev().collect();
    let y_flip: Vec<f64> = y.iter().copied().rev().collect();
    let mask_flip: Vec<bool> = limit_free_mask.iter().copied().rev().collect();
    let flipped_start = n - 1 - start_index;
    let peak_flip: Vec<usize> = peak_inds.iter().map(|&p| n - 1 - p).collect();

    let (mut limited_flip, clipped_flip, regions_flip) = limited_ltr_slope(
        &x_flip, &y_flip, max_slope, max_slope_decay, flipped_start,
        &peak_flip, &mask_flip, concha_interference,
    );

    // Flip back
    limited_flip.reverse();
    let clipped: Vec<bool> = clipped_flip.into_iter().rev().collect();
    let regions: Vec<[usize; 2]> = regions_flip.iter().map(|r| {
        [n - 1 - r[1], n - 1 - r[0]]
    }).collect();

    (limited_flip, clipped, regions)
}

/// Format frequency for EqualizerAPO output
fn format_frequency(f: f64) -> String {
    if f >= 1000.0 {
        format!("{:.0}k", f / 1000.0)
    } else {
        format!("{:.0}", f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_frequency_response() {
        let fr = FrequencyResponse::new("test", vec![20.0, 100.0, 1000.0], vec![1.0, 2.0, 3.0]).unwrap();
        assert_eq!(fr.name, "test");
        assert_eq!(fr.frequency.len(), 3);
    }

    #[test]
    fn test_interpolate() {
        let mut fr = FrequencyResponse::new("test", vec![20.0, 100.0, 1000.0, 10000.0], vec![1.0, 2.0, 3.0, 4.0]).unwrap();
        fr.interpolate(None, 1.1, 20.0, 10000.0).unwrap();
        assert!(fr.frequency.len() > 3);
        // Values should be between 1.0 and 4.0
        for &v in &fr.raw {
            assert!(v >= 1.0 - 1e-10 && v <= 4.0 + 1e-10, "v = {}", v);
        }
    }

    #[test]
    fn test_center() {
        let mut fr = FrequencyResponse::new("test", vec![100.0, 1000.0, 10000.0], vec![5.0, 3.0, 1.0]).unwrap();
        let offset = fr.center(1000.0);
        assert!((fr.raw[1]).abs() < 1e-10, "raw[1] = {}", fr.raw[1]);
    }

    #[test]
    fn test_smoothen() {
        let f = utils::generate_frequencies(20.0, 20000.0, 1.01);
        let raw: Vec<f64> = f.iter().map(|&fi| (fi / 1000.0).ln() * 5.0).collect();
        let mut fr = FrequencyResponse::new("test", f, raw).unwrap();
        fr.smoothen(1.0 / 12.0, 2.0, 6000.0, 8000.0);
        assert_eq!(fr.smoothed.len(), fr.frequency.len());
    }

    #[test]
    fn test_process_pipeline() {
        let f = utils::generate_frequencies(20.0, 20000.0, 1.01);
        let raw: Vec<f64> = f.iter().map(|&fi| {
            if fi < 200.0 { 3.0 } else if fi < 2000.0 { 0.0 } else { -2.0 }
        }).collect();
        let mut fr = FrequencyResponse::new("test", f, raw).unwrap();
        let target = FrequencyResponse::new("flat", vec![], vec![]).unwrap();
        let result = fr.process(&target, 0.0, 105.0, 0.7, 0.0, 10000.0, 0.7, 0.0, 44100.0,
                               6.0, 18.0, false, 1.0 / 12.0, 2.0, 6000.0, 8000.0, 1.0);
        assert!(!result.equalization.is_empty());
        assert!(!fr.equalization.is_empty());
    }

    #[test]
    fn test_protection_mask() {
        let y = vec![0.0, -1.0, 0.0, -5.0, 0.0, -1.0, 0.0];
        let peaks = vec![0, 2, 4, 6];
        let dips = vec![1, 3, 5];
        let mask = protection_mask(&y, &peaks, &dips);
        assert_eq!(mask.len(), y.len());
        // Deep dip at index 3 should have protection around it
    }

    #[test]
    fn test_eqapo_graphic_eq() {
        let f = utils::generate_frequencies(20.0, 20000.0, 1.01);
        let eq: Vec<f64> = f.iter().map(|&fi| (fi / 1000.0).ln()).collect();
        let mut fr = FrequencyResponse::new("test", f, vec![0.0; 693]).unwrap();
        fr.equalization = eq;
        let geq = fr.eqapo_graphic_eq(true, 0.0, DEFAULT_GRAPHIC_EQ_STEP);
        assert!(geq.starts_with("GraphicEQ:"));
    }

    #[test]
    fn debug_equalize() {
        let csv_bytes = std::fs::read("../test_file/ZERO.txt").unwrap();
        let csv_text = String::from_utf8_lossy(&csv_bytes).to_string();
        let parsed = crate::csv::parse_csv(&csv_text).unwrap();
        let mut fr = FrequencyResponse::new("ZERO", parsed.frequency, parsed.raw).unwrap();
        fr.interpolate(None, DEFAULT_STEP, DEFAULT_F_MIN, DEFAULT_F_MAX).unwrap();
        fr.center(1000.0);

        let target = FrequencyResponse::new("flat", fr.frequency.clone(), vec![0.0; fr.frequency.len()]).unwrap();
        fr.compensate(&target, 0.0, 105.0, 0.7, 0.0, 10000.0, 0.7, 0.0, 44100.0);

        // Debug compensate step by step
        let mut target_fr = target.clone();
        let _ = target_fr.interpolate(Some(&fr.frequency), DEFAULT_STEP, DEFAULT_F_MIN, DEFAULT_F_MAX);
        println!("\nBefore center target:");
        println!("  target_fr.raw[0..5]: {:?}", &target_fr.raw[..5]);
        let center_val = target_fr.center(1000.0);
        println!("  center offset: {:.6}", center_val);
        println!("  target_fr.raw after center[0..5]: {:?}", &target_fr.raw[..5]);

        let bass_shelf = biquad_response_low_shelf(&fr.frequency, 44100.0, 105.0, 0.7, 0.0);
        let treble_shelf = biquad_response_high_shelf(&fr.frequency, 44100.0, 10000.0, 0.7, 0.0);
        let tilt_curve = utils::log_tilt(&fr.frequency, 0.0);
        println!("  bass_shelf[0..3]: {:?}", &bass_shelf[..3]);
        println!("  treble_shelf[0..3]: {:?}", &treble_shelf[..3]);
        println!("  tilt[0..3]: {:?}", &tilt_curve[..3]);

        println!("\nAfter compensate:");
        println!("  error[0..5]: {:?}", &fr.error[..5]);
        println!("  target[0..5]: {:?}", &fr.target[..5]);
        println!("  raw[0..5]: {:?}", &fr.raw[..5]);

        fr.smoothen(DEFAULT_SMOOTHING_WINDOW_SIZE, DEFAULT_TREBLE_SMOOTHING_WINDOW_SIZE,
                   DEFAULT_TREBLE_SMOOTHING_F_LOWER, DEFAULT_TREBLE_SMOOTHING_F_UPPER);

        println!("After smoothen:");
        println!("  smoothed[0..5]: {:?}", &fr.smoothed[..5]);
        println!("  error_smoothed[0..5]: {:?}", &fr.error_smoothed[..5]);

        // Check equalization range
        fr.equalize(6.0, 18.0, 0.0, false,
                   DEFAULT_SMOOTHING_WINDOW_SIZE, DEFAULT_TREBLE_SMOOTHING_WINDOW_SIZE,
                   DEFAULT_TREBLE_SMOOTHING_F_LOWER, DEFAULT_TREBLE_SMOOTHING_F_UPPER, 1.0);

        println!("After equalize:");
        println!("  eq[0..5]: {:?}", &fr.equalization[..5]);
        let eq_min = fr.equalization.iter().cloned().fold(f64::INFINITY, f64::min);
        let eq_max = fr.equalization.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        println!("  eq range: {:.6} to {:.6}", eq_min, eq_max);
    }

    #[test]
    fn test_compare_with_python() {
        // Python reference results for ZERO.txt:
        // n=695, f_first=20.0, f_last=19955.536162
        // raw_first5=[2.792699, 2.814828, 2.838021, 2.861685, 2.885859]
        // raw_min=-11.518618, raw_max=8.30868
        // smoothed_first5=[2.792143, 2.815361, 2.838588, 2.861822, 2.885831]
        // smoothed_min=-6.755851, smoothed_max=8.297614
        // eq_first5=[-2.795209, -2.817264, -2.839167, -2.860918, -2.882518]
        // eq_min=-8.295483, eq_max=6.03237

        let csv_bytes = std::fs::read("../test_file/ZERO.txt").unwrap();
        let csv_text = String::from_utf8_lossy(&csv_bytes).to_string();
        let parsed = crate::csv::parse_csv(&csv_text).unwrap();

        let mut fr = FrequencyResponse::new("ZERO", parsed.frequency, parsed.raw).unwrap();
        fr.interpolate(None, DEFAULT_STEP, DEFAULT_F_MIN, DEFAULT_F_MAX).unwrap();
        fr.center(1000.0);

        // Check basic structure
        assert_eq!(fr.frequency.len(), 695);
        assert!((fr.frequency[0] - 20.0).abs() < 0.01);
        assert!((fr.frequency[694] - 19955.536162).abs() < 1.0);

        // Check raw after center (tolerance: 0.01 dB)
        let raw_py = [2.792699, 2.814828, 2.838021, 2.861685, 2.885859];
        for i in 0..5 {
            assert!((fr.raw[i] - raw_py[i]).abs() < 0.01,
                "raw[{}]: rust={:.6} python={:.6} diff={:.6}",
                i, fr.raw[i], raw_py[i], (fr.raw[i] - raw_py[i]).abs());
        }

        // Check smoothed (tolerance: 0.1 dB - savgol implementation may differ slightly)
        fr.smoothen(DEFAULT_SMOOTHING_WINDOW_SIZE, DEFAULT_TREBLE_SMOOTHING_WINDOW_SIZE,
                   DEFAULT_TREBLE_SMOOTHING_F_LOWER, DEFAULT_TREBLE_SMOOTHING_F_UPPER);
        let sm_py = [2.792143, 2.815361, 2.838588, 2.861822, 2.885831];
        for i in 0..5 {
            assert!((fr.smoothed[i] - sm_py[i]).abs() < 0.1,
                "smoothed[{}]: rust={:.6} python={:.6} diff={:.6}",
                i, fr.smoothed[i], sm_py[i], (fr.smoothed[i] - sm_py[i]).abs());
        }

        // Check equalization (tolerance: 1.0 dB - accumulated differences)
        let target = FrequencyResponse::new("flat", fr.frequency.clone(), vec![0.0; fr.frequency.len()]).unwrap();
        fr.compensate(&target, 0.0, 105.0, 0.7, 0.0, 10000.0, 0.7, 0.0, 44100.0);
        fr.equalize(6.0, 18.0, 0.0, false,
                   DEFAULT_SMOOTHING_WINDOW_SIZE, DEFAULT_TREBLE_SMOOTHING_WINDOW_SIZE,
                   DEFAULT_TREBLE_SMOOTHING_F_LOWER, DEFAULT_TREBLE_SMOOTHING_F_UPPER, 1.0);

        let eq_py = [-2.795209, -2.817264, -2.839167, -2.860918, -2.882518];
        for i in 0..5 {
            assert!((fr.equalization[i] - eq_py[i]).abs() < 1.0,
                "eq[{}]: rust={:.6} python={:.6} diff={:.6}",
                i, fr.equalization[i], eq_py[i], (fr.equalization[i] - eq_py[i]).abs());
        }

        // Print comparison for manual inspection
        println!("\n=== Rust vs Python Comparison (ZERO.txt) ===");
        println!("Points: {} vs 695", fr.frequency.len());
        println!("Freq: {:.6} vs 20.0", fr.frequency[0]);
        println!("Raw[0]: {:.6} vs 2.792699 (diff: {:.6})", fr.raw[0], (fr.raw[0] - 2.792699).abs());
        println!("Raw[100]: {:.6}", fr.raw[100]);
        println!("Smoothed[0]: {:.6} vs 2.792143 (diff: {:.6})", fr.smoothed[0], (fr.smoothed[0] - 2.792143).abs());
        println!("Eq[0]: {:.6} vs -2.795209 (diff: {:.6})", fr.equalization[0], (fr.equalization[0] - -2.795209).abs());
        let eq_min = fr.equalization.iter().cloned().fold(f64::INFINITY, f64::min);
        let eq_max = fr.equalization.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        println!("Eq range: {:.6} to {:.6} vs -8.295483 to 6.03237", eq_min, eq_max);
    }
}
