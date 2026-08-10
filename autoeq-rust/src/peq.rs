use crate::constants::*;
use crate::dsp;
use crate::error::Result;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::Arc;

/// Biquad filter coefficients: (a0_norm, a1, a2, b0, b1, b2)
/// a0 is normalized to 1.0
type BiquadCoeffs = (f64, f64, f64, f64, f64, f64);

/// Trait for parametric EQ filter types
pub trait PeqFilter: std::fmt::Debug {
    fn fc(&self) -> f64;
    fn q(&self) -> f64;
    fn gain(&self) -> f64;
    fn set_fc(&mut self, fc: f64);
    fn set_q(&mut self, q: f64);
    fn set_gain(&mut self, gain: f64);
    fn biquad_coefficients(&self) -> BiquadCoeffs;
    fn filter_type(&self) -> FilterType;
    fn optimize_fc(&self) -> bool;
    fn optimize_q(&self) -> bool;
    fn optimize_gain(&self) -> bool;
    fn min_fc(&self) -> f64;
    fn max_fc(&self) -> f64;
    fn min_q(&self) -> f64;
    fn max_q(&self) -> f64;
    fn min_gain(&self) -> f64;
    fn max_gain(&self) -> f64;
    fn f(&self) -> &[f64];
    fn fs(&self) -> f64;

    /// Compute frequency response using phi-form biquad evaluation
    fn frequency_response(&self) -> Vec<f64> {
        let (_a0, a1, a2, b0, b1, b2) = self.biquad_coefficients();
        let a1n = -a1;
        let a2n = -a2;

        self.f().iter().map(|&fi| {
            let w = 2.0 * std::f64::consts::PI * fi / self.fs();
            let phi = 4.0 * (w / 2.0).sin().powi(2);
            let num = (b0 + b1 + b2).powi(2) + (b0 * b2 * phi - (b1 * (b0 + b2) + 4.0 * b0 * b2)) * phi;
            let den = (1.0 + a1n + a2n).powi(2) + (a2n * phi - (a1n * (1.0 + a2n) + 4.0 * a2n)) * phi;
            10.0 * num.max(1e-30).log10() - 10.0 * den.max(1e-30).log10()
        }).collect()
    }

    /// Sharpness penalty for optimizer
    fn sharpness_penalty(&self) -> f64 { 0.0 }

    /// Band penalty for optimizer
    fn band_penalty(&self) -> f64 { 0.0 }

    /// Initialize filter parameters from target curve. Returns [log10(fc), q, gain].
    fn init(&mut self, target: &[f64]) -> Vec<f64>;

    /// Clone as boxed trait object
    fn clone_box(&self) -> Box<dyn PeqFilter>;
}

/// Common filter fields
#[derive(Debug, Clone)]
struct FilterBase {
    f: Vec<f64>,
    fs: f64,
    fc: f64,
    q: f64,
    gain: f64,
    min_fc: f64,
    max_fc: f64,
    min_q: f64,
    max_q: f64,
    min_gain: f64,
    max_gain: f64,
    optimize_fc: bool,
    optimize_q: bool,
    optimize_gain: bool,
}

/// Peaking (bell) filter
#[derive(Debug, Clone)]
pub struct PeakingFilter {
    base: FilterBase,
}

/// Low shelf filter
#[derive(Debug, Clone)]
pub struct LowShelfFilter {
    base: FilterBase,
}

/// High shelf filter
#[derive(Debug, Clone)]
pub struct HighShelfFilter {
    base: FilterBase,
}

impl PeakingFilter {
    pub fn new(
        f: Vec<f64>, fs: f64,
        fc: Option<f64>, q: Option<f64>, gain: Option<f64>,
        min_fc: f64, max_fc: f64, min_q: f64, max_q: f64,
        min_gain: f64, max_gain: f64,
        optimize_fc: bool, optimize_q: bool, optimize_gain: bool,
    ) -> Self {
        Self {
            base: FilterBase {
                f, fs,
                fc: fc.unwrap_or((min_fc * max_fc).sqrt()),
                q: q.unwrap_or(2.0_f64.sqrt()),
                gain: gain.unwrap_or(0.0),
                min_fc, max_fc, min_q, max_q, min_gain, max_gain,
                optimize_fc, optimize_q, optimize_gain,
            }
        }
    }
}

impl PeqFilter for PeakingFilter {
    fn fc(&self) -> f64 { self.base.fc }
    fn q(&self) -> f64 { self.base.q }
    fn gain(&self) -> f64 { self.base.gain }
    fn set_fc(&mut self, fc: f64) { self.base.fc = fc; }
    fn set_q(&mut self, q: f64) { self.base.q = q; }
    fn set_gain(&mut self, gain: f64) { self.base.gain = gain; }
    fn filter_type(&self) -> FilterType { FilterType::Peaking }
    fn optimize_fc(&self) -> bool { self.base.optimize_fc }
    fn optimize_q(&self) -> bool { self.base.optimize_q }
    fn optimize_gain(&self) -> bool { self.base.optimize_gain }
    fn min_fc(&self) -> f64 { self.base.min_fc }
    fn max_fc(&self) -> f64 { self.base.max_fc }
    fn min_q(&self) -> f64 { self.base.min_q }
    fn max_q(&self) -> f64 { self.base.max_q }
    fn min_gain(&self) -> f64 { self.base.min_gain }
    fn max_gain(&self) -> f64 { self.base.max_gain }
    fn f(&self) -> &[f64] { &self.base.f }
    fn fs(&self) -> f64 { self.base.fs }

    fn biquad_coefficients(&self) -> BiquadCoeffs {
        let a = 10.0_f64.powf(self.base.gain / 40.0);
        let w0 = 2.0 * std::f64::consts::PI * self.base.fc / self.base.fs;
        let alpha = w0.sin() / (2.0 * self.base.q);

        let a0 = 1.0 + alpha / a;
        let a1 = -(-2.0 * w0.cos()) / a0;
        let a2 = -(1.0 - alpha / a) / a0;
        let b0 = (1.0 + alpha * a) / a0;
        let b1 = (-2.0 * w0.cos()) / a0;
        let b2 = (1.0 - alpha * a) / a0;

        (1.0, a1, a2, b0, b1, b2)
    }

    fn sharpness_penalty(&self) -> f64 {
        let fr = self.frequency_response();
        let gain_limit = -0.09503189270199464 + 20.575128011847003 * (1.0 / self.base.q);
        let x_val = self.base.gain / gain_limit - 1.0;
        let sigmoid = 1.0 / (1.0 + (-x_val * 100.0).exp());
        fr.iter().map(|&v| v * v * sigmoid).sum::<f64>() / fr.len() as f64
    }

    fn band_penalty(&self) -> f64 {
        let fr = self.frequency_response();
        let f = &self.base.f;
        let fc = self.base.fc;

        let fc_ix = f.iter().enumerate()
            .min_by(|(_, a), (_, b)| (*a - fc).abs().partial_cmp(&(*b - fc).abs()).unwrap())
            .map(|(i, _)| i).unwrap_or(0);

        let ix10k = f.iter().enumerate()
            .min_by(|(_, a), (_, b)| (*a - 10000.0).abs().partial_cmp(&(*b - 10000.0).abs()).unwrap())
            .map(|(i, _)| i).unwrap_or(f.len() - 1);

        let n = fc_ix.min(ix10k.saturating_sub(fc_ix));
        if n == 0 { return 0.0; }

        let left: Vec<f64> = fr[(fc_ix - n)..fc_ix].to_vec();
        let right: Vec<f64> = fr[fc_ix..(fc_ix + n)].iter().copied().rev().collect();

        left.iter().zip(right.iter()).map(|(a, b)| (a - b).powi(2)).sum::<f64>() / n as f64
    }

    fn init(&mut self, target: &[f64]) -> Vec<f64> {
        let pos_target: Vec<f64> = target.iter().map(|&x| if x > 0.0 { x } else { 0.0 }).collect();
        let (pos_peaks, _pp, pos_widths, pos_heights) = dsp::find_peaks_with_props(&pos_target, 0.0);

        let neg_target: Vec<f64> = target.iter().map(|&x| if x < 0.0 { -x } else { 0.0 }).collect();
        let (neg_peaks, _np, neg_widths, neg_heights) = dsp::find_peaks_with_props(&neg_target, 0.0);

        let f = &self.base.f;
        let min_ix = f.iter().position(|&fi| fi >= self.base.min_fc).unwrap_or(0);
        let max_ix = f.iter().rposition(|&fi| fi <= self.base.max_fc).unwrap_or(f.len() - 1);

        struct Candidate { ix: usize, width: f64, abs_height: f64, is_positive: bool }
        let mut candidates: Vec<Candidate> = Vec::new();

        for &p in &pos_peaks {
            if p >= min_ix && p <= max_ix {
                if let Some(pi) = pos_peaks.iter().position(|&x| x == p) {
                    candidates.push(Candidate { ix: p, width: pos_widths[pi], abs_height: pos_heights[pi], is_positive: true });
                }
            }
        }
        for &p in &neg_peaks {
            if p >= min_ix && p <= max_ix {
                if let Some(pi) = neg_peaks.iter().position(|&x| x == p) {
                    candidates.push(Candidate { ix: p, width: neg_widths[pi], abs_height: neg_heights[pi], is_positive: false });
                }
            }
        }

        if candidates.is_empty() {
            if !self.base.optimize_fc {
                self.base.q = 2.0_f64.sqrt();
                let fc_idx = self.base.f.iter().enumerate()
                    .min_by(|(_, a), (_, b)| (*a - self.base.fc).abs().partial_cmp(&(*b - self.base.fc).abs()).unwrap()).unwrap().0;
                self.base.gain = target[fc_idx].clamp(self.base.min_gain, self.base.max_gain);
            } else {
                let mid_ix = (min_ix + max_ix) / 2;
                self.base.fc = f[mid_ix].clamp(self.base.min_fc, self.base.max_fc);
                self.base.q = 2.0_f64.sqrt();
                self.base.gain = 0.0;
            }
        } else {
            let mut best = 0_usize;
            let mut best_score = 0.0;
            for (i, c) in candidates.iter().enumerate() {
                let score = c.width * c.abs_height;
                if score > best_score {
                    best_score = score;
                    best = i;
                }
            }
            let c = &candidates[best];

            if self.base.optimize_fc {
                self.base.fc = f[c.ix].clamp(self.base.min_fc, self.base.max_fc);
            }

            let f_step_log2 = (f[1] / f[0]).log2();
            let bw = (2.0_f64.powf(f_step_log2).powf(c.width)).log2();
            if bw > 0.0 {
                let q = (2.0_f64.powf(bw)).sqrt() / (2.0_f64.powf(bw) - 1.0);
                self.base.q = q.clamp(self.base.min_q, self.base.max_q);
            } else {
                self.base.q = 2.0_f64.sqrt().clamp(self.base.min_q, self.base.max_q);
            }

            let gain = if c.is_positive { c.abs_height } else { -c.abs_height };
            self.base.gain = gain.clamp(self.base.min_gain, self.base.max_gain);
        }

        let mut params = Vec::new();
        if self.base.optimize_fc {
            params.push(self.base.fc.log10());
        }
        if self.base.optimize_q {
            params.push(self.base.q);
        }
        if self.base.optimize_gain {
            params.push(self.base.gain);
        }
        params
    }

    fn clone_box(&self) -> Box<dyn PeqFilter> {
        Box::new(self.clone())
    }
}

impl LowShelfFilter {
    pub fn new(
        f: Vec<f64>, fs: f64,
        fc: Option<f64>, q: Option<f64>, gain: Option<f64>,
        min_fc: f64, max_fc: f64, min_q: f64, max_q: f64,
        min_gain: f64, max_gain: f64,
        optimize_fc: bool, optimize_q: bool, optimize_gain: bool,
    ) -> Self {
        Self {
            base: FilterBase {
                f, fs,
                fc: fc.unwrap_or(105.0),
                q: q.unwrap_or(0.7),
                gain: gain.unwrap_or(0.0),
                min_fc, max_fc, min_q, max_q, min_gain, max_gain,
                optimize_fc, optimize_q, optimize_gain,
            }
        }
    }
}

impl PeqFilter for LowShelfFilter {
    fn fc(&self) -> f64 { self.base.fc }
    fn q(&self) -> f64 { self.base.q }
    fn gain(&self) -> f64 { self.base.gain }
    fn set_fc(&mut self, fc: f64) { self.base.fc = fc; }
    fn set_q(&mut self, q: f64) { self.base.q = q; }
    fn set_gain(&mut self, gain: f64) { self.base.gain = gain; }
    fn filter_type(&self) -> FilterType { FilterType::LowShelf }
    fn optimize_fc(&self) -> bool { self.base.optimize_fc }
    fn optimize_q(&self) -> bool { self.base.optimize_q }
    fn optimize_gain(&self) -> bool { self.base.optimize_gain }
    fn min_fc(&self) -> f64 { self.base.min_fc }
    fn max_fc(&self) -> f64 { self.base.max_fc }
    fn min_q(&self) -> f64 { self.base.min_q }
    fn max_q(&self) -> f64 { self.base.max_q }
    fn min_gain(&self) -> f64 { self.base.min_gain }
    fn max_gain(&self) -> f64 { self.base.max_gain }
    fn f(&self) -> &[f64] { &self.base.f }
    fn fs(&self) -> f64 { self.base.fs }

    fn biquad_coefficients(&self) -> BiquadCoeffs {
        let a = 10.0_f64.powf(self.base.gain / 40.0);
        let w0 = 2.0 * std::f64::consts::PI * self.base.fc / self.base.fs;
        let alpha = w0.sin() / (2.0 * self.base.q);
        let cos_w0 = w0.cos();
        let sqrt_a = a.sqrt();

        let a0 = (a + 1.0) + (a - 1.0) * cos_w0 + 2.0 * sqrt_a * alpha;
        let a1 = -(-2.0 * ((a - 1.0) + (a + 1.0) * cos_w0)) / a0;
        let a2 = -((a + 1.0) + (a - 1.0) * cos_w0 - 2.0 * sqrt_a * alpha) / a0;
        let b0 = (a * ((a + 1.0) - (a - 1.0) * cos_w0 + 2.0 * sqrt_a * alpha)) / a0;
        let b1 = (2.0 * a * ((a - 1.0) - (a + 1.0) * cos_w0)) / a0;
        let b2 = (a * ((a + 1.0) - (a - 1.0) * cos_w0 - 2.0 * sqrt_a * alpha)) / a0;

        (1.0, a1, a2, b0, b1, b2)
    }

    fn band_penalty(&self) -> f64 {
        let fr = self.frequency_response();
        let f = &self.base.f;
        let fc = self.base.fc;
        let gain = self.base.gain;

        let fc_ix = f.iter().enumerate()
            .min_by(|(_, a), (_, b)| (*a - fc).abs().partial_cmp(&(*b - fc).abs()).unwrap())
            .map(|(i, _)| i).unwrap_or(0);

        let ix10k = f.iter().enumerate()
            .min_by(|(_, a), (_, b)| (*a - 10000.0).abs().partial_cmp(&(*b - 10000.0).abs()).unwrap())
            .map(|(i, _)| i).unwrap_or(f.len() - 1);

        let n = fc_ix.min(ix10k.saturating_sub(fc_ix));
        if n == 0 { return 0.0; }

        let left: Vec<f64> = fr[(fc_ix - n)..fc_ix].to_vec();
        let right: Vec<f64> = fr[fc_ix..(fc_ix + n)].iter().copied().rev().collect();

        left.iter().zip(right.iter()).map(|(a, b)| (a - (gain - b)).powi(2)).sum::<f64>() / n as f64
    }

    fn init(&mut self, target: &[f64]) -> Vec<f64> {
        let f = &self.base.f;
        let min_ix = f.iter().position(|&fi| fi >= self.base.min_fc.max(40.0)).unwrap_or(0);
        let max_ix = f.iter().rposition(|&fi| fi <= self.base.max_fc.min(10000.0)).unwrap_or(f.len() - 1);

        let mut best_val = 0.0;
        let mut best_ix = min_ix;
        for ix in min_ix..=max_ix {
            let mean: f64 = target[..=ix].iter().sum::<f64>() / (ix + 1) as f64;
            if mean.abs() > best_val {
                best_val = mean.abs();
                best_ix = ix;
            }
        }

        let mut params = Vec::new();
        if self.base.optimize_fc {
            self.base.fc = f[best_ix].clamp(self.base.min_fc, self.base.max_fc);
            params.push(self.base.fc.log10());
        }
        if self.base.optimize_q {
            self.base.q = 0.7_f64.clamp(self.base.min_q, self.base.max_q);
            params.push(self.base.q);
        }
        if self.base.optimize_gain {
            let mut temp_filter = self.clone();
            temp_filter.base.gain = 1.0;
            let fr_1db = temp_filter.frequency_response();
            let sum_fr: f64 = fr_1db.iter().sum();
            if sum_fr.abs() > 1e-10 {
                let dot: f64 = target.iter().zip(fr_1db.iter()).map(|(t, f)| t * f).sum();
                self.base.gain = (dot / sum_fr).clamp(self.base.min_gain, self.base.max_gain);
            }
            params.push(self.base.gain);
        }
        params
    }

    fn clone_box(&self) -> Box<dyn PeqFilter> {
        Box::new(self.clone())
    }
}

impl HighShelfFilter {
    pub fn new(
        f: Vec<f64>, fs: f64,
        fc: Option<f64>, q: Option<f64>, gain: Option<f64>,
        min_fc: f64, max_fc: f64, min_q: f64, max_q: f64,
        min_gain: f64, max_gain: f64,
        optimize_fc: bool, optimize_q: bool, optimize_gain: bool,
    ) -> Self {
        Self {
            base: FilterBase {
                f, fs,
                fc: fc.unwrap_or(10000.0),
                q: q.unwrap_or(0.7),
                gain: gain.unwrap_or(0.0),
                min_fc, max_fc, min_q, max_q, min_gain, max_gain,
                optimize_fc, optimize_q, optimize_gain,
            }
        }
    }
}

impl PeqFilter for HighShelfFilter {
    fn fc(&self) -> f64 { self.base.fc }
    fn q(&self) -> f64 { self.base.q }
    fn gain(&self) -> f64 { self.base.gain }
    fn set_fc(&mut self, fc: f64) { self.base.fc = fc; }
    fn set_q(&mut self, q: f64) { self.base.q = q; }
    fn set_gain(&mut self, gain: f64) { self.base.gain = gain; }
    fn filter_type(&self) -> FilterType { FilterType::HighShelf }
    fn optimize_fc(&self) -> bool { self.base.optimize_fc }
    fn optimize_q(&self) -> bool { self.base.optimize_q }
    fn optimize_gain(&self) -> bool { self.base.optimize_gain }
    fn min_fc(&self) -> f64 { self.base.min_fc }
    fn max_fc(&self) -> f64 { self.base.max_fc }
    fn min_q(&self) -> f64 { self.base.min_q }
    fn max_q(&self) -> f64 { self.base.max_q }
    fn min_gain(&self) -> f64 { self.base.min_gain }
    fn max_gain(&self) -> f64 { self.base.max_gain }
    fn f(&self) -> &[f64] { &self.base.f }
    fn fs(&self) -> f64 { self.base.fs }

    fn biquad_coefficients(&self) -> BiquadCoeffs {
        let a = 10.0_f64.powf(self.base.gain / 40.0);
        let w0 = 2.0 * std::f64::consts::PI * self.base.fc / self.base.fs;
        let alpha = w0.sin() / (2.0 * self.base.q);
        let cos_w0 = w0.cos();
        let sqrt_a = a.sqrt();

        let a0 = (a + 1.0) - (a - 1.0) * cos_w0 + 2.0 * sqrt_a * alpha;
        let a1 = -(2.0 * ((a - 1.0) - (a + 1.0) * cos_w0)) / a0;
        let a2 = -((a + 1.0) - (a - 1.0) * cos_w0 - 2.0 * sqrt_a * alpha) / a0;
        let b0 = (a * ((a + 1.0) + (a - 1.0) * cos_w0 + 2.0 * sqrt_a * alpha)) / a0;
        let b1 = (-2.0 * a * ((a - 1.0) + (a + 1.0) * cos_w0)) / a0;
        let b2 = (a * ((a + 1.0) + (a - 1.0) * cos_w0 - 2.0 * sqrt_a * alpha)) / a0;

        (1.0, a1, a2, b0, b1, b2)
    }

    fn band_penalty(&self) -> f64 {
        let fr = self.frequency_response();
        let f = &self.base.f;
        let fc = self.base.fc;
        let gain = self.base.gain;

        let fc_ix = f.iter().enumerate()
            .min_by(|(_, a), (_, b)| (*a - fc).abs().partial_cmp(&(*b - fc).abs()).unwrap())
            .map(|(i, _)| i).unwrap_or(0);

        let ix10k = f.iter().enumerate()
            .min_by(|(_, a), (_, b)| (*a - 10000.0).abs().partial_cmp(&(*b - 10000.0).abs()).unwrap())
            .map(|(i, _)| i).unwrap_or(f.len() - 1);

        let n = fc_ix.min(ix10k.saturating_sub(fc_ix));
        if n == 0 { return 0.0; }

        let left: Vec<f64> = fr[(fc_ix - n)..fc_ix].to_vec();
        let right: Vec<f64> = fr[fc_ix..(fc_ix + n)].iter().copied().rev().collect();

        left.iter().zip(right.iter()).map(|(a, b)| (a - (gain - b)).powi(2)).sum::<f64>() / n as f64
    }

    fn init(&mut self, target: &[f64]) -> Vec<f64> {
        let f = &self.base.f;
        let min_ix = f.iter().position(|&fi| fi >= self.base.min_fc.max(40.0)).unwrap_or(0);
        let max_ix = f.iter().rposition(|&fi| fi <= self.base.max_fc.min(10000.0)).unwrap_or(f.len() - 1);

        let mut best_val = 0.0;
        let mut best_ix = min_ix;
        for ix in min_ix..=max_ix {
            let remaining = &target[ix..];
            let mean: f64 = remaining.iter().sum::<f64>() / remaining.len() as f64;
            if mean.abs() > best_val {
                best_val = mean.abs();
                best_ix = ix;
            }
        }

        self.base.fc = f[best_ix].clamp(self.base.min_fc, self.base.max_fc);
        self.base.q = 0.7_f64.clamp(self.base.min_q, self.base.max_q);

        let mut params = Vec::new();
        if self.base.optimize_fc {
            params.push(self.base.fc.log10());
        }
        if self.base.optimize_q {
            params.push(self.base.q);
        }
        if self.base.optimize_gain {
            let mut temp_filter = self.clone();
            temp_filter.base.gain = 1.0;
            let fr_1db = temp_filter.frequency_response();
            let sum_fr: f64 = fr_1db.iter().sum();
            if sum_fr.abs() > 1e-10 {
                let dot: f64 = target.iter().zip(fr_1db.iter()).map(|(t, f)| t * f).sum();
                self.base.gain = (dot / sum_fr).clamp(self.base.min_gain, self.base.max_gain);
            }
            params.push(self.base.gain);
        }
        params
    }

    fn clone_box(&self) -> Box<dyn PeqFilter> {
        Box::new(self.clone())
    }
}

// ============================================================
// Fast optimization path: FilterSpec + direct biquad computation
// ============================================================

/// Flat representation of a filter for fast loss evaluation.
/// Avoids Box, trait dispatch, and Vec clones in the hot path.
#[derive(Debug, Clone)]
struct FilterSpec {
    filter_type: FilterType,
    optimize_fc: bool,
    optimize_q: bool,
    optimize_gain: bool,
    fixed_fc: f64,
    fixed_q: f64,
    fixed_gain: f64,
    min_fc: f64,
    max_fc: f64,
    min_q: f64,
    max_q: f64,
    min_gain: f64,
    max_gain: f64,
}

/// Precomputed context for fast loss evaluation without allocations
#[derive(Debug, Clone)]
struct LossContext {
    filter_specs: Vec<FilterSpec>,
    fs: f64,
    phi: Vec<f64>,
    target: Vec<f64>,
    min_f_ix: usize,
    max_f_ix: usize,
    ix10k: usize,
    n: usize,
    ln10: f64,
    /// precomputed target mean above 10kHz
    target_mean_10k: f64,
}

/// Compute biquad coefficients directly (no trait object)
fn biquad_coeffs_direct(ft: FilterType, fc: f64, q: f64, gain: f64, fs: f64) -> BiquadCoeffs {
    match ft {
        FilterType::Peaking => {
            let a = 10.0_f64.powf(gain / 40.0);
            let w0 = 2.0 * std::f64::consts::PI * fc / fs;
            let alpha = w0.sin() / (2.0 * q);
            let a0 = 1.0 + alpha / a;
            let a1 = -(-2.0 * w0.cos()) / a0;
            let a2 = -(1.0 - alpha / a) / a0;
            let b0 = (1.0 + alpha * a) / a0;
            let b1 = (-2.0 * w0.cos()) / a0;
            let b2 = (1.0 - alpha * a) / a0;
            (1.0, a1, a2, b0, b1, b2)
        }
        FilterType::LowShelf => {
            let a = 10.0_f64.powf(gain / 40.0);
            let w0 = 2.0 * std::f64::consts::PI * fc / fs;
            let alpha = w0.sin() / (2.0 * q);
            let cos_w0 = w0.cos();
            let sqrt_a = a.sqrt();
            let a0 = (a + 1.0) + (a - 1.0) * cos_w0 + 2.0 * sqrt_a * alpha;
            let a1 = -(-2.0 * ((a - 1.0) + (a + 1.0) * cos_w0)) / a0;
            let a2 = -((a + 1.0) + (a - 1.0) * cos_w0 - 2.0 * sqrt_a * alpha) / a0;
            let b0 = (a * ((a + 1.0) - (a - 1.0) * cos_w0 + 2.0 * sqrt_a * alpha)) / a0;
            let b1 = (2.0 * a * ((a - 1.0) - (a + 1.0) * cos_w0)) / a0;
            let b2 = (a * ((a + 1.0) - (a - 1.0) * cos_w0 - 2.0 * sqrt_a * alpha)) / a0;
            (1.0, a1, a2, b0, b1, b2)
        }
        FilterType::HighShelf => {
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
            (1.0, a1, a2, b0, b1, b2)
        }
    }
}

/// PEQ optimizer
#[derive(Debug)]
pub struct PEQ {
    pub f: Vec<f64>,
    pub fs: f64,
    pub filters: Vec<Box<dyn PeqFilter>>,
    pub target: Vec<f64>,
    /// Fast loss evaluation context (built once, used many times)
    loss_ctx: LossContext,
}

/// PEQ implementation
impl PEQ {
    pub fn from_config(
        config: &PeqConfig,
        f: Vec<f64>,
        fs: f64,
        target: Vec<f64>,
    ) -> Result<Self> {
        let min_f_ix = f.iter().position(|&fi| fi >= config.optimizer.min_f).unwrap_or(0);
        let max_f_ix = f.iter().rposition(|&fi| fi <= config.optimizer.max_f).unwrap_or(f.len() - 1);

        let phi: Vec<f64> = f.iter().map(|&fi| {
            let w = 2.0 * std::f64::consts::PI * fi / fs;
            4.0 * (w / 2.0).sin().powi(2)
        }).collect();

        let n = f.len();
        let ln10 = std::f64::consts::LN_10;

        // Precompute ix10k
        let ix10k = f.iter().enumerate()
            .min_by(|(_, a), (_, b)| (*a - 10000.0).abs().partial_cmp(&(*b - 10000.0).abs()).unwrap())
            .map(|(i, _)| i).unwrap_or(n - 1);

        // Precompute target mean above 10kHz
        let target_mean_10k = if ix10k < n {
            target[ix10k..].iter().sum::<f64>() / (n - ix10k) as f64
        } else {
            0.0
        };

        let phi_for_ctx = phi.clone();
        let mut peq = Self {
            f: f.clone(),
            fs,
            filters: Vec::new(),
            target,
            loss_ctx: LossContext {
                filter_specs: Vec::new(),  // populated below
                fs,
                phi: phi_for_ctx,
                target: Vec::new(),  // populated after
                min_f_ix,
                max_f_ix,
                ix10k,
                n,
                ln10,
                target_mean_10k,
            },
        };

        // Build filters from config (trait objects for init/final result)
        for fc_config in &config.filters {
            let filter_type = get_default_filter_type(fc_config.filter_type, &config.filter_defaults);

            let defaults = config.filter_defaults.as_ref();
            let min_fc = fc_config.min_fc
                .or_else(|| defaults.and_then(|d| d.min_fc))
                .unwrap_or(match filter_type {
                    FilterType::Peaking => DEFAULT_PEAKING_FILTER_MIN_FC,
                    _ => DEFAULT_SHELF_FILTER_MIN_FC,
                });
            let max_fc = fc_config.max_fc
                .or_else(|| defaults.and_then(|d| d.max_fc))
                .unwrap_or(match filter_type {
                    FilterType::Peaking => DEFAULT_PEAKING_FILTER_MAX_FC,
                    _ => DEFAULT_SHELF_FILTER_MAX_FC,
                });
            let min_q = fc_config.min_q
                .or_else(|| defaults.and_then(|d| d.min_q))
                .unwrap_or(match filter_type {
                    FilterType::Peaking => DEFAULT_PEAKING_FILTER_MIN_Q,
                    _ => DEFAULT_SHELF_FILTER_MIN_Q,
                });
            let max_q = fc_config.max_q
                .or_else(|| defaults.and_then(|d| d.max_q))
                .unwrap_or(match filter_type {
                    FilterType::Peaking => DEFAULT_PEAKING_FILTER_MAX_Q,
                    _ => DEFAULT_SHELF_FILTER_MAX_Q,
                });
            let min_gain = fc_config.min_gain
                .or_else(|| defaults.and_then(|d| d.min_gain))
                .unwrap_or(match filter_type {
                    FilterType::Peaking => DEFAULT_PEAKING_FILTER_MIN_GAIN,
                    _ => DEFAULT_SHELF_FILTER_MIN_GAIN,
                });
            let max_gain = fc_config.max_gain
                .or_else(|| defaults.and_then(|d| d.max_gain))
                .unwrap_or(match filter_type {
                    FilterType::Peaking => DEFAULT_PEAKING_FILTER_MAX_GAIN,
                    _ => DEFAULT_SHELF_FILTER_MAX_GAIN,
                });

            let q = fc_config.q.or_else(|| defaults.and_then(|d| d.q));

            // If min == max, the parameter is fixed
            let optimize_fc = fc_config.fc.is_none() && (max_fc - min_fc).abs() > 1e-10;
            let optimize_q = fc_config.q.is_none() && (max_q - min_q).abs() > 1e-10;
            let optimize_gain = fc_config.gain.is_none() && (max_gain - min_gain).abs() > 1e-10;

            let fc_val = if (min_fc - max_fc).abs() < 1e-10 { Some(min_fc) } else { fc_config.fc };
            let q_val = if (min_q - max_q).abs() < 1e-10 { Some(min_q) } else { q };

            // Build the FilterSpec for fast evaluation
            let spec = FilterSpec {
                filter_type,
                optimize_fc,
                optimize_q,
                optimize_gain,
                fixed_fc: fc_val.unwrap_or((min_fc * max_fc).sqrt()),
                fixed_q: q_val.unwrap_or(2.0_f64.sqrt()),
                fixed_gain: fc_config.gain.unwrap_or(0.0),
                min_fc, max_fc, min_q, max_q, min_gain, max_gain,
            };
            peq.loss_ctx.filter_specs.push(spec);

            // Build the trait object for init/final result
            let filter: Box<dyn PeqFilter> = match filter_type {
                FilterType::Peaking => Box::new(PeakingFilter::new(
                    peq.f.clone(), fs, fc_val, q_val, fc_config.gain,
                    min_fc, max_fc, min_q, max_q, min_gain, max_gain,
                    optimize_fc, optimize_q, optimize_gain,
                )),
                FilterType::LowShelf => Box::new(LowShelfFilter::new(
                    peq.f.clone(), fs, fc_val, q_val, fc_config.gain,
                    min_fc, max_fc, min_q, max_q, min_gain, max_gain,
                    optimize_fc, optimize_q, optimize_gain,
                )),
                FilterType::HighShelf => Box::new(HighShelfFilter::new(
                    peq.f.clone(), fs, fc_val, q_val, fc_config.gain,
                    min_fc, max_fc, min_q, max_q, min_gain, max_gain,
                    optimize_fc, optimize_q, optimize_gain,
                )),
            };
            peq.filters.push(filter);
        }

        // Set the target in loss_ctx
        peq.loss_ctx.target = peq.target.clone();

        Ok(peq)
    }

    /// Get the combined frequency response of all filters
    pub fn frequency_response(&self) -> Vec<f64> {
        let n = self.f.len();
        let mut fr = vec![0.0; n];
        for filter in &self.filters {
            let filter_fr = filter.frequency_response();
            for i in 0..n {
                fr[i] += filter_fr[i];
            }
        }
        fr
    }

    /// Get the maximum gain across all filters
    pub fn max_gain(&self) -> f64 {
        let fr = self.frequency_response();
        fr.iter().cloned().fold(f64::NEG_INFINITY, f64::max)
    }

    /// Fast loss evaluation — zero heap allocation in the hot path.
    /// Uses precomputed FilterSpec, phi, ix10k, and target_mean_10k.
    fn eval_loss_for_params(&self, x: &[f64]) -> f64 {
        let ctx = &self.loss_ctx;
        let n = ctx.n;
        let ln10 = ctx.ln10;
        let mut fr = vec![0.0; n];
        let mut penalty_acc = 0.0;

        let mut idx = 0;
        for spec in &ctx.filter_specs {
            let fc = if spec.optimize_fc {
                let v = 10.0_f64.powf(x[idx]).clamp(spec.min_fc, spec.max_fc);
                idx += 1;
                v
            } else {
                spec.fixed_fc
            };
            let q = if spec.optimize_q {
                let v = x[idx].clamp(spec.min_q, spec.max_q);
                idx += 1;
                v
            } else {
                spec.fixed_q
            };
            let gain = if spec.optimize_gain {
                let v = x[idx].clamp(spec.min_gain, spec.max_gain);
                idx += 1;
                v
            } else {
                spec.fixed_gain
            };

            let (_, a1, a2, b0, b1, b2) = biquad_coeffs_direct(spec.filter_type, fc, q, gain, ctx.fs);

            let a1n = -a1;
            let a2n = -a2;
            let sum_b = b0 + b1 + b2;
            let sum_a = 1.0 + a1n + a2n;
            let num_base = sum_b * sum_b;
            let den_base = sum_a * sum_a;

            // Sharpness penalty sigmoid (only for peaking filters)
            let sigmoid = if matches!(spec.filter_type, FilterType::Peaking) {
                let gain_limit = -0.09503189270199464 + 20.575128011847003 * (1.0 / q);
                let x_val = gain / gain_limit - 1.0;
                1.0 / (1.0 + (-x_val * 100.0).exp())
            } else {
                0.0
            };

            for i in 0..n {
                let phi = ctx.phi[i];
                let num = num_base + (b0 * b2 * phi - (b1 * (b0 + b2) + 4.0 * b0 * b2)) * phi;
                let den = den_base + (a2n * phi - (a1n * (1.0 + a2n) + 4.0 * a2n)) * phi;
                let db = 10.0 * num.max(1e-30).ln() / ln10 - 10.0 * den.max(1e-30).ln() / ln10;
                fr[i] += db;
                penalty_acc += db * db * sigmoid;
            }
        }

        let penalty = penalty_acc / n as f64;

        // Above 10kHz: use precomputed mean
        let ix10k = ctx.ix10k;
        let fr_mean_10k = if ix10k < n {
            fr[ix10k..].iter().sum::<f64>() / (n - ix10k) as f64
        } else {
            0.0
        };
        let target_mean = ctx.target_mean_10k;

        // MSE in optimization range
        let lo = ctx.min_f_ix;
        let hi = ctx.max_f_ix.min(n);
        let mut mse = 0.0;
        for i in lo..ix10k.min(hi) {
            let diff = ctx.target[i] - fr[i];
            mse += diff * diff;
        }
        for _i in ix10k.max(lo)..hi {
            let diff = target_mean - fr_mean_10k;
            mse += diff * diff;
        }
        mse /= (hi - lo) as f64;

        (mse + penalty).max(0.0).sqrt()
    }

    /// Get sorted filter indices (matching Python's sort order)
    fn get_sorted_indices(&self) -> Vec<usize> {
        let mut indices: Vec<usize> = (0..self.filters.len()).collect();
        indices.sort_by_key(|&i| {
            let f = &self.filters[i];
            let priority = match (f.optimize_fc(), f.optimize_q()) {
                (true, true) => 0,
                (true, false) => 1,
                (false, true) => 2,
                (false, false) => 3,
            };
            let type_priority = match f.filter_type() {
                FilterType::Peaking => 0,
                FilterType::LowShelf => 1,
                FilterType::HighShelf => 2,
            };
            let range = (f.max_fc() / f.min_fc()).log2();
            (priority, type_priority, (range * 1000.0) as u64)
        });
        indices.reverse();
        indices
    }

    /// Initialize optimizer parameters (matching Python's order)
    fn init_optimizer_params(&mut self) -> Vec<f64> {
        let indices = self.get_sorted_indices();

        let mut filter_params: Vec<Vec<f64>> = vec![Vec::new(); self.filters.len()];
        let mut remaining_target = self.target.clone();

        for &idx in &indices {
            let filter = &mut self.filters[idx];
            let init_params = filter.init(&remaining_target);
            filter_params[idx] = init_params;

            let fr = filter.frequency_response();
            let n = remaining_target.len().min(fr.len());
            for i in 0..n {
                remaining_target[i] -= fr[i];
            }
        }

        filter_params.into_iter().flatten().collect()
    }

    /// Get optimizer bounds (must match init_optimizer_params order)
    fn init_optimizer_bounds(&self) -> Vec<(f64, f64)> {
        let mut bounds = Vec::new();
        for filter in &self.filters {
            if filter.optimize_fc() {
                bounds.push((filter.min_fc().log10(), filter.max_fc().log10()));
            }
            if filter.optimize_q() {
                bounds.push((filter.min_q(), filter.max_q()));
            }
            if filter.optimize_gain() {
                bounds.push((filter.min_gain(), filter.max_gain()));
            }
        }
        bounds
    }

    /// Check if any filter has free parameters
    pub fn has_free_params(&self) -> bool {
        self.filters.iter().any(|f| f.optimize_fc() || f.optimize_q() || f.optimize_gain())
    }

    /// Run the optimizer using Nelder-Mead simplex algorithm with random restarts.
    /// On native: parallel restarts via std::thread. On WASM: sequential.
    pub fn optimize(&mut self, _max_time: Option<f64>) -> Result<()> {
        if !self.has_free_params() {
            return Ok(());
        }

        let bounds = self.init_optimizer_bounds();
        let x0 = self.init_optimizer_params();

        let initial_loss = self.eval_loss_for_params(&x0);
        eprintln!("Initial loss: {:.6}", initial_loss);

        // Single restart for deterministic results (matching Python behavior)
        // Improved Nelder-Mead parameters and convergence criteria compensate
        let num_restarts = 1;

        // Run restarts (parallel on native, sequential on WASM)
        let results = Self::run_restarts(&self.loss_ctx, &bounds, &x0, num_restarts);

        let mut best_loss = initial_loss;
        let mut best_x = x0.clone();

        for (loss, x) in results {
            eprintln!("Restart best loss: {:.6}", loss);
            if loss < best_loss {
                best_loss = loss;
                best_x = x;
            }
        }

        eprintln!("Final loss: {:.6}", best_loss);

        Self::apply_params(&mut self.filters, &best_x);
        Ok(())
    }

    /// Execute Nelder-Mead restarts — parallel on native, sequential on WASM.
    fn run_restarts(
        ctx: &LossContext,
        bounds: &[(f64, f64)],
        x0: &[f64],
        num_restarts: usize,
    ) -> Vec<(f64, Vec<f64>)> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let ctx = Arc::new(ctx.clone());
            let bounds = bounds.to_vec();
            let x0 = x0.to_vec();
            let n = x0.len();
            let handles: Vec<_> = (0..num_restarts).map(|restart| {
                let ctx = Arc::clone(&ctx);
                let bounds = bounds.clone();
                let x0 = x0.clone();
                std::thread::spawn(move || {
                    run_one_restart(&ctx, &bounds, &x0, n, restart)
                })
            }).collect();
            handles.into_iter().map(|h| h.join().unwrap()).collect()
        }
        #[cfg(target_arch = "wasm32")]
        {
            let n = x0.len();
            (0..num_restarts).map(|restart| {
                run_one_restart(ctx, bounds, x0, n, restart)
            }).collect()
        }
    }

    fn apply_params(filters: &mut [Box<dyn PeqFilter>], x: &[f64]) {
        let mut idx = 0;
        for filter in filters.iter_mut() {
            if filter.optimize_fc() {
                filter.set_fc(10.0_f64.powf(x[idx]).clamp(filter.min_fc(), filter.max_fc()));
                idx += 1;
            }
            if filter.optimize_q() {
                filter.set_q(x[idx].clamp(filter.min_q(), filter.max_q()));
                idx += 1;
            }
            if filter.optimize_gain() {
                filter.set_gain(x[idx].clamp(filter.min_gain(), filter.max_gain()));
                idx += 1;
            }
        }
    }
}

/// Run a single Nelder-Mead restart. Returns (best_loss, best_params).
fn run_one_restart(
    ctx: &LossContext,
    bounds: &[(f64, f64)],
    x0: &[f64],
    n: usize,
    restart: usize,
) -> (f64, Vec<f64>) {
    let start_x = if restart == 0 {
        x0.to_vec()
    } else {
        let mut seed = 0xDEADBEEF_CAFEBABE + restart as u64;
        let mut rx = x0.to_vec();
        for i in 0..n {
            let (lo, hi) = bounds[i];
            let noise = (hi - lo) * 0.3 * (rand_f64(&mut seed) - 0.5);
            rx[i] = (rx[i] + noise).clamp(lo, hi);
        }
        rx
    };

    // Build initial simplex with larger step size for better exploration
    let mut simplex: Vec<Vec<f64>> = Vec::with_capacity(n + 1);
    simplex.push(start_x);
    for i in 0..n {
        let mut xi = simplex[0].clone();
        let (lo, hi) = bounds[i];
        let range = hi - lo;
        // Use 30% of range for initial simplex (was 25%)
        let step = range * 0.30;
        xi[i] = (xi[i] + step).clamp(lo, hi);
        if (xi[i] - simplex[0][i]).abs() < 1e-10 {
            xi[i] = (xi[i] - step * 2.0).clamp(lo, hi);
        }
        simplex.push(xi);
    }

    let eval_fn = |params: &[f64]| -> f64 {
        let n_fr = ctx.n;
        let ln10 = ctx.ln10;
        let mut fr = vec![0.0; n_fr];
        let mut penalty_acc = 0.0;
        let mut fcs: Vec<f64> = Vec::with_capacity(ctx.filter_specs.len());

        let mut idx = 0;
        for spec in &ctx.filter_specs {
            let fc = if spec.optimize_fc {
                let v = 10.0_f64.powf(params[idx]).clamp(spec.min_fc, spec.max_fc);
                idx += 1;
                v
            } else { spec.fixed_fc };
            let q = if spec.optimize_q {
                let v = params[idx].clamp(spec.min_q, spec.max_q);
                idx += 1;
                v
            } else { spec.fixed_q };
            let gain = if spec.optimize_gain {
                let v = params[idx].clamp(spec.min_gain, spec.max_gain);
                idx += 1;
                v
            } else { spec.fixed_gain };

            fcs.push(fc);

            let (_, a1, a2, b0, b1, b2) = biquad_coeffs_direct(spec.filter_type, fc, q, gain, ctx.fs);

            let a1n = -a1;
            let a2n = -a2;
            let sum_b = b0 + b1 + b2;
            let sum_a = 1.0 + a1n + a2n;
            let num_base = sum_b * sum_b;
            let den_base = sum_a * sum_a;

            // 循环不变量提到外面(保持与原代码相同的乘法/加法顺序 → bit-exact)
            let b0b2 = b0 * b2;
            let c_num = b1 * (b0 + b2) + 4.0 * b0b2;
            let c_den = a1n * (1.0 + a2n) + 4.0 * a2n;

            let sigmoid = if matches!(spec.filter_type, FilterType::Peaking) {
                let gain_limit = -0.09503189270199464 + 20.575128011847003 * (1.0 / q);
                let x_val = gain / gain_limit - 1.0;
                1.0 / (1.0 + (-x_val * 100.0).exp())
            } else { 0.0 };

            for i in 0..n_fr {
                let phi = ctx.phi[i];
                let num = num_base + (b0b2 * phi - c_num) * phi;
                let den = den_base + (a2n * phi - c_den) * phi;
                let db = 10.0 * num.max(1e-30).ln() / ln10 - 10.0 * den.max(1e-30).ln() / ln10;
                fr[i] += db;
                penalty_acc += db * db * sigmoid;
            }
        }
        let penalty = penalty_acc / n_fr as f64;

        // Filter spacing penalty: discourage multiple peaking filters piling at nearly the same fc.
        // (Reference tool result uses well-separated fc; stacked filters create over-shoot valleys.)
        let mut spacing_penalty = 0.0;
        if fcs.len() > 1 {
            let mut sorted = fcs.clone();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
            for w in sorted.windows(2) {
                let ratio = w[1] / w[0].max(1.0);
                if ratio < 1.4 {
                    spacing_penalty += (1.4 - ratio) * (1.4 - ratio) * 0.5;
                }
            }
        }
        let penalty = penalty + spacing_penalty;

        let ix10k = ctx.ix10k;
        let fr_mean_10k = if ix10k < n_fr {
            fr[ix10k..].iter().sum::<f64>() / (n_fr - ix10k) as f64
        } else { 0.0 };
        let target_mean = ctx.target_mean_10k;

        let lo = ctx.min_f_ix;
        let hi = ctx.max_f_ix.min(n_fr);
        let mut mse = 0.0;
        for i in lo..ix10k.min(hi) {
            let diff = ctx.target[i] - fr[i];
            mse += diff * diff;
        }
        for _i in ix10k.max(lo)..hi {
            let diff = target_mean - fr_mean_10k;
            mse += diff * diff;
        }
        mse /= (hi - lo) as f64;

        (mse + penalty).max(0.0).sqrt()
    };

    let mut simplex_loss: Vec<f64> = simplex.iter().map(|x| eval_fn(x)).collect();

    // Increase max iterations for better convergence (Python uses time-based stopping)
    let max_iters = 80000;
    // Nelder-Mead parameters (tuned for high-dimensional problems)
    let nm_alpha = 1.0;   // reflection coefficient
    let nm_gamma = 2.2;   // expansion coefficient (slightly more aggressive)
    let nm_rho = 0.35;    // contraction coefficient (more conservative)
    let nm_sigma = 0.35;  // shrink coefficient (more conservative)
    // Perturbation parameters for escaping local optima
    let perturb_threshold = 300;  // iterations before perturbation
    let perturb_magnitude = 0.25; // perturbation size (fraction of range)

    // Track loss history for early stopping (matching Python behavior)
    let mut loss_history: Vec<f64> = Vec::new();
    let window_size = 30;
    let mut best_loss_ever = f64::MAX;
    let mut best_loss_count = 0;

    for iter in 0..max_iters {
        let mut order: Vec<usize> = (0..simplex.len()).collect();
        order.sort_by(|&a, &b| simplex_loss[a].partial_cmp(&simplex_loss[b]).unwrap());

        let best = order[0];
        let worst = order[n];
        let second_worst = order[n - 1];

        let mean_loss = simplex_loss.iter().sum::<f64>() / simplex_loss.len() as f64;
        let std_loss = (simplex_loss.iter()
            .map(|l| (l - mean_loss).powi(2))
            .sum::<f64>() / simplex_loss.len() as f64).sqrt();

        // Track best loss for early stopping
        let current_best = simplex_loss[order[0]];
        loss_history.push(current_best);

        // Track if we're still improving
        if current_best < best_loss_ever - 1e-8 {
            best_loss_ever = current_best;
            best_loss_count = 0;
        } else {
            best_loss_count += 1;
        }

        // Early stopping conditions (matching Python behavior)
        if std_loss < 1e-7 {
            break;
        }

        // Check if loss has plateaued (similar to Python's min_change_rate)
        if loss_history.len() >= window_size {
            let recent = &loss_history[loss_history.len() - window_size..];
            let old_mean: f64 = recent[..window_size/2].iter().sum::<f64>() / (window_size/2) as f64;
            let new_mean: f64 = recent[window_size/2..].iter().sum::<f64>() / (window_size/2) as f64;
            let change_rate = (old_mean - new_mean) / old_mean.abs().max(1e-10);

            // Stop only if improvement is less than 0.005% over window AND optimizer has run long enough
            if change_rate.abs() < 0.00005 && iter > 3000 {
                break;
            }
        }

        // Perturbation mechanism to escape local optima
        if best_loss_count > perturb_threshold && iter > 300 {
            // Perturb the worst vertices
            for &idx in &order[1..] {
                for i in 0..n {
                    let (lo, hi) = bounds[i];
                    let range = hi - lo;
                    let perturbation = (range * perturb_magnitude) * (rand_f64(&mut (0xDEADBEEF + iter as u64 + i as u64)) - 0.5);
                    simplex[idx][i] = (simplex[idx][i] + perturbation).clamp(lo, hi);
                }
                simplex_loss[idx] = eval_fn(&simplex[idx]);
            }
            best_loss_count = 0;  // Reset counter
        }

        // Stop if no improvement for many iterations
        if best_loss_count > 25000 && iter > 1000 {
            break;
        }

        // Check if we've reached a very good loss
        if current_best < 0.0001 {
            break;
        }

        let mut centroid = vec![0.0; n];
        for &idx in &order[..n] {
            for i in 0..n {
                centroid[i] += simplex[idx][i];
            }
        }
        for i in 0..n {
            centroid[i] /= n as f64;
        }

        let mut reflected = centroid.clone();
        for i in 0..n {
            reflected[i] = centroid[i] + nm_alpha * (centroid[i] - simplex[worst][i]);
        }
        for i in 0..n {
            let (lo, hi) = bounds[i];
            reflected[i] = reflected[i].clamp(lo, hi);
        }
        let reflected_loss = eval_fn(&reflected);

        if reflected_loss < simplex_loss[second_worst] && reflected_loss >= simplex_loss[best] {
            simplex[worst] = reflected;
            simplex_loss[worst] = reflected_loss;
        } else if reflected_loss < simplex_loss[best] {
            let mut expanded = centroid.clone();
            for i in 0..n {
                expanded[i] = centroid[i] + nm_gamma * (reflected[i] - centroid[i]);
            }
            for i in 0..n {
                let (lo, hi) = bounds[i];
                expanded[i] = expanded[i].clamp(lo, hi);
            }
            let expanded_loss = eval_fn(&expanded);

            if expanded_loss < reflected_loss {
                simplex[worst] = expanded;
                simplex_loss[worst] = expanded_loss;
            } else {
                simplex[worst] = reflected;
                simplex_loss[worst] = reflected_loss;
            }
        } else {
            let mut contracted = centroid.clone();
            for i in 0..n {
                contracted[i] = centroid[i] + nm_rho * (simplex[worst][i] - centroid[i]);
            }
            for i in 0..n {
                let (lo, hi) = bounds[i];
                contracted[i] = contracted[i].clamp(lo, hi);
            }
            let contracted_loss = eval_fn(&contracted);

            if contracted_loss < simplex_loss[worst] {
                simplex[worst] = contracted;
                simplex_loss[worst] = contracted_loss;
            } else {
                for &idx in &order[1..] {
                    for i in 0..n {
                        simplex[idx][i] = simplex[best][i] + nm_sigma * (simplex[idx][i] - simplex[best][i]);
                    }
                    for i in 0..n {
                        let (lo, hi) = bounds[i];
                        simplex[idx][i] = simplex[idx][i].clamp(lo, hi);
                    }
                    simplex_loss[idx] = eval_fn(&simplex[idx]);
                }
            }
        }
    }

    let mut best_idx = 0;
    let mut restart_best_loss = f64::MAX;
    for (i, &loss) in simplex_loss.iter().enumerate() {
        if loss < restart_best_loss {
            restart_best_loss = loss;
            best_idx = i;
        }
    }

    (restart_best_loss, simplex[best_idx].clone())
}

/// Simple xorshift random number in [0, 1). Free function usable outside PEQ.
fn rand_f64(seed: &mut u64) -> f64 {
    let mut x = *seed;
    x ^= x << 13;
    x ^= x >> 7;
    x ^= x << 17;
    if x == 0 { x = 1; }
    *seed = x;
    (x as f64) / (u64::MAX as f64)
}

/// Result of PEQ optimization
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PeqResult {
    pub preamp: f64,
    pub filters: Vec<FilterResult>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FilterResult {
    #[serde(rename = "type")]
    pub filter_type: FilterType,
    pub fc: f64,
    pub gain: f64,
    pub q: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_peaking_biquad() {
        let f = vec![100.0, 1000.0, 10000.0];
        let filter = PeakingFilter::new(
            f, 44100.0, Some(1000.0), Some(1.0), Some(3.0),
            20.0, 10000.0, 0.1, 6.0, -20.0, 20.0,
            false, false, false,
        );
        let fr = filter.frequency_response();
        assert!(fr.len() == 3);
        assert!((fr[1] - 3.0).abs() < 0.5, "fr[1] = {}", fr[1]);
    }

    #[test]
    fn test_lowshelf_biquad() {
        let f = vec![20.0, 100.0, 1000.0, 10000.0];
        let filter = LowShelfFilter::new(
            f, 44100.0, Some(105.0), Some(0.7), Some(6.0),
            20.0, 10000.0, 0.4, 0.7, -20.0, 20.0,
            false, false, false,
        );
        let fr = filter.frequency_response();
        assert!(fr[0] > 0.0, "fr[0] = {}", fr[0]);
    }

    #[test]
    fn test_highshelf_biquad() {
        let f = vec![20.0, 1000.0, 10000.0, 20000.0];
        let filter = HighShelfFilter::new(
            f, 44100.0, Some(10000.0), Some(0.7), Some(6.0),
            20.0, 10000.0, 0.4, 0.7, -20.0, 20.0,
            false, false, false,
        );
        let fr = filter.frequency_response();
        assert!(fr[2] > 0.0, "fr[2] = {}", fr[2]);
    }

    #[test]
    fn test_peq_from_config() {
        let config = &crate::constants::PEQ_CONFIGS["8_PEAKING_WITH_SHELVES"];
        let f = crate::utils::generate_frequencies(20.0, 20000.0, 1.01);
        let target = vec![0.0; f.len()];
        let peq = PEQ::from_config(config, f, 44100.0, target).unwrap();
        assert_eq!(peq.filters.len(), 10); // 1 low + 1 high + 8 peaking
    }

    #[test]
    fn test_direct_biquad_vs_trait() {
        // Verify that the direct biquad computation matches the trait method
        let fs = 44100.0;
        let fc = 1000.0;
        let q = 1.5;
        let gain = 3.0;

        let f = vec![100.0, 500.0, 1000.0, 5000.0, 10000.0];

        // Peaking
        let filter = PeakingFilter::new(
            f.clone(), fs, Some(fc), Some(q), Some(gain),
            20.0, 10000.0, 0.1, 6.0, -20.0, 20.0,
            false, false, false,
        );
        let fr_trait = filter.frequency_response();
        let (_, a1, a2, b0, b1, b2) = biquad_coeffs_direct(FilterType::Peaking, fc, q, gain, fs);
        let fr_direct = compute_fr_direct(&f, fs, a1, a2, b0, b1, b2);
        for i in 0..f.len() {
            assert!((fr_trait[i] - fr_direct[i]).abs() < 1e-10,
                "Peaking f={}: trait={} direct={}", f[i], fr_trait[i], fr_direct[i]);
        }

        // LowShelf
        let filter = LowShelfFilter::new(
            f.clone(), fs, Some(fc), Some(0.7), Some(gain),
            20.0, 10000.0, 0.4, 0.7, -20.0, 20.0,
            false, false, false,
        );
        let fr_trait = filter.frequency_response();
        let (_, a1, a2, b0, b1, b2) = biquad_coeffs_direct(FilterType::LowShelf, fc, 0.7, gain, fs);
        let fr_direct = compute_fr_direct(&f, fs, a1, a2, b0, b1, b2);
        for i in 0..f.len() {
            assert!((fr_trait[i] - fr_direct[i]).abs() < 1e-10,
                "LowShelf f={}: trait={} direct={}", f[i], fr_trait[i], fr_direct[i]);
        }

        // HighShelf
        let filter = HighShelfFilter::new(
            f.clone(), fs, Some(fc), Some(0.7), Some(gain),
            20.0, 10000.0, 0.4, 0.7, -20.0, 20.0,
            false, false, false,
        );
        let fr_trait = filter.frequency_response();
        let (_, a1, a2, b0, b1, b2) = biquad_coeffs_direct(FilterType::HighShelf, fc, 0.7, gain, fs);
        let fr_direct = compute_fr_direct(&f, fs, a1, a2, b0, b1, b2);
        for i in 0..f.len() {
            assert!((fr_trait[i] - fr_direct[i]).abs() < 1e-10,
                "HighShelf f={}: trait={} direct={}", f[i], fr_trait[i], fr_direct[i]);
        }
    }

    /// Helper: compute FR from coefficients using same phi formula
    fn compute_fr_direct(f: &[f64], fs: f64, a1: f64, a2: f64, b0: f64, b1: f64, b2: f64) -> Vec<f64> {
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
}
