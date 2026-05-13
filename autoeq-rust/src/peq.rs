use crate::constants::*;
use crate::dsp;
use crate::error::Result;

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
        // In the Python code, a1 and a2 from biquad_coefficients() are negated in the fr property
        // The formula uses: den = (1 + (-a1) + (-a2))^2 + ((-a2)*phi - ((-a1)*(1+(-a2)) + 4*(-a2)))*phi
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

        // Find index closest to fc
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
        // 匹配 Python: 找正峰 (target clamped >= 0) 和负峰 (-target clamped >= 0)
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

            // Compute Q from width (matching Python: bw = log2(2^(f_step_log2 * width)))
            let f_step_log2 = (f[1] / f[0]).log2();
            let bw = (2.0_f64.powf(f_step_log2).powf(c.width)).log2();
            if bw > 0.0 {
                let q = (2.0_f64.powf(bw)).sqrt() / (2.0_f64.powf(bw) - 1.0);
                self.base.q = q.clamp(self.base.min_q, self.base.max_q);
            } else {
                self.base.q = 2.0_f64.sqrt().clamp(self.base.min_q, self.base.max_q);
            }

            // Gain from peak height (positive peak → positive gain, negative → negative)
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

        // Find fc by scanning: maximize abs(mean(target[:ix+1]))
        let mut best_val = 0.0;
        let mut best_ix = min_ix;
        for ix in min_ix..=max_ix {
            let mean: f64 = target[..=ix].iter().sum::<f64>() / (ix + 1) as f64;
            if mean.abs() > best_val {
                best_val = mean.abs();
                best_ix = ix;
            }
        }

        // 匹配 Python: 仅当 optimize_fc=true 时覆盖 fc
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
            // Estimate gain using weighted average with 1 dB shelf
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

        // Find fc by scanning: maximize abs(mean(target[ix:]))
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

        // 始终设置 fc 和 q（用于 gain 估计和频率响应减法），优化时按需返回
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
            // Estimate gain
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

/// PEQ optimizer
#[derive(Debug)]
pub struct PEQ {
    pub f: Vec<f64>,
    pub fs: f64,
    pub filters: Vec<Box<dyn PeqFilter>>,
    pub target: Vec<f64>,
    min_f_ix: usize,
    max_f_ix: usize,
    optimizer_config: OptimizerConfig,
    phi: Vec<f64>,
}

impl PEQ {
    pub fn from_config(
        config: &PeqConfig,
        f: Vec<f64>,
        fs: f64,
        target: Vec<f64>,
    ) -> Result<Self> {
        let min_f_ix = f.iter().position(|&fi| fi >= config.optimizer.min_f).unwrap_or(0);
        let max_f_ix = f.iter().rposition(|&fi| fi <= config.optimizer.max_f).unwrap_or(f.len() - 1);

        // 预计算 phi 值（只依赖频率，不依赖滤波器参数）
        let phi: Vec<f64> = f.iter().map(|&fi| {
            let w = 2.0 * std::f64::consts::PI * fi / fs;
            4.0 * (w / 2.0).sin().powi(2)
        }).collect();

        let mut peq = Self {
            f, fs,
            filters: Vec::new(),
            target,
            min_f_ix,
            max_f_ix,
            optimizer_config: config.optimizer.clone(),
            phi,
        };

        // Build filters from config
        for fc_config in &config.filters {
            let filter_type = get_default_filter_type(fc_config.filter_type, &config.filter_defaults);

            // Merge defaults
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
        // 直接使用已实现的frequency_response方法
        let fr = self.frequency_response();
        fr.iter().cloned().fold(f64::NEG_INFINITY, f64::max)
    }

    /// Compute optimizer loss (RMSE + penalties) - 高性能版本，使用预计算 phi 避免克隆
    fn optimizer_loss(&mut self, params: &[f64]) -> f64 {
        // 保存所有滤波器原始参数
        let original: Vec<(f64, f64, f64)> = self.filters.iter()
            .map(|f| (f.fc(), f.q(), f.gain()))
            .collect();

        // 临时设置测试参数
        let mut idx = 0;
        for filter in self.filters.iter_mut() {
            if filter.optimize_fc() {
                filter.set_fc(10.0_f64.powf(params[idx]).clamp(filter.min_fc(), filter.max_fc()));
                idx += 1;
            }
            if filter.optimize_q() {
                filter.set_q(params[idx].clamp(filter.min_q(), filter.max_q()));
                idx += 1;
            }
            if filter.optimize_gain() {
                filter.set_gain(params[idx].clamp(filter.min_gain(), filter.max_gain()));
                idx += 1;
            }
        }

        // 使用预计算 phi 和内联 biquad 评估计算级联频率响应
        let n = self.f.len();
        let mut fr = vec![0.0; n];
        let ln10 = std::f64::consts::LN_10;
        for filter in &self.filters {
            let (_, a1, a2, b0, b1, b2) = filter.biquad_coefficients();
            let a1n = -a1;
            let a2n = -a2;
            let sum_b = b0 + b1 + b2;
            let sum_a = 1.0 + a1n + a2n;
            let num_base = sum_b * sum_b;
            let den_base = sum_a * sum_a;

            for i in 0..n {
                let phi = self.phi[i];
                let num = num_base + (b0 * b2 * phi - (b1 * (b0 + b2) + 4.0 * b0 * b2)) * phi;
                let den = den_base + (a2n * phi - (a1n * (1.0 + a2n) + 4.0 * a2n)) * phi;
                fr[i] += 10.0 * num.max(1e-30).ln() / ln10 - 10.0 * den.max(1e-30).ln() / ln10;
            }
        }

        // 恢复原始参数
        for (i, filter) in self.filters.iter_mut().enumerate() {
            let (fc, q, gain) = original[i];
            if filter.optimize_fc() {
                filter.set_fc(fc);
            }
            if filter.optimize_q() {
                filter.set_q(q);
            }
            if filter.optimize_gain() {
                filter.set_gain(gain);
            }
        }

        // Above 10kHz: replace with mean
        let ix10k = self.f.iter().enumerate()
            .min_by(|(_, a), (_, b)| (*a - 10000.0).abs().partial_cmp(&(*b - 10000.0).abs()).unwrap())
            .map(|(i, _)| i).unwrap_or(n - 1);

        let mut target_adj = self.target.clone();
        let mut fr_adj = fr.clone();

        if ix10k < n {
            let target_mean: f64 = target_adj[ix10k..].iter().sum::<f64>() / (n - ix10k) as f64;
            let fr_mean: f64 = fr_adj[ix10k..].iter().sum::<f64>() / (n - ix10k) as f64;
            for i in ix10k..n {
                target_adj[i] = target_mean;
                fr_adj[i] = fr_mean;
            }
        }

        // MSE in optimization range
        let lo = self.min_f_ix;
        let hi = self.max_f_ix.min(n);
        let mse: f64 = target_adj[lo..hi].iter().zip(fr_adj[lo..hi].iter())
            .map(|(t, f)| (t - f).powi(2))
            .sum::<f64>() / (hi - lo) as f64;

        let penalty: f64 = self.filters.iter().map(|f| f.sharpness_penalty()).sum();

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
        // Python版本使用reverse=True排序
        indices.reverse();
        indices
    }

    /// Initialize optimizer parameters (matching Python's order)
    fn init_optimizer_params(&mut self) -> Vec<f64> {
        let indices = self.get_sorted_indices();

        // Initialize filter params as Vec<Vec<f64>>, one per filter
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

        // Flatten params in original filter order (matching Python)
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

    /// Simple xorshift random number in [0, 1)
    fn rand_f64() -> f64 {
        use std::cell::Cell;
        thread_local! {
            static STATE: Cell<u64> = Cell::new(0xDEADBEEF_CAFEBABE);
        }
        STATE.with(|s| {
            let mut x = s.get();
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            if x == 0 { x = 1; }
            s.set(x);
            (x as f64) / (u64::MAX as f64)
        })
    }

    /// Run the optimizer using Nelder-Mead simplex algorithm with random restarts
    pub fn optimize(&mut self, _max_time: Option<f64>) -> Result<()> {
        if !self.has_free_params() {
            return Ok(());
        }

        let bounds = self.init_optimizer_bounds();
        let x0: Vec<f64> = self.init_optimizer_params();
        let n = x0.len();

        let initial_loss = self.eval_loss_for_params(&x0);
        eprintln!("Initial loss: {:.6}", initial_loss);

        let clamp = |x: &mut [f64]| {
            for (i, xi) in x.iter_mut().enumerate() {
                let (lo, hi) = bounds[i];
                *xi = xi.clamp(lo, hi);
            }
        };

        let mut best_x = x0.clone();
        let mut best_loss = initial_loss;

        let num_restarts = 3;

        for restart in 0..num_restarts {
            let start_x = if restart == 0 {
                x0.clone()
            } else {
                let mut rx = best_x.clone();
                for i in 0..n {
                    let (lo, hi) = bounds[i];
                    let noise = (hi - lo) * 0.3 * (Self::rand_f64() - 0.5);
                    rx[i] = (rx[i] + noise).clamp(lo, hi);
                }
                rx
            };

            // Build initial simplex
            let mut simplex: Vec<Vec<f64>> = Vec::with_capacity(n + 1);
            simplex.push(start_x);
            for i in 0..n {
                let mut xi = simplex[0].clone();
                let (lo, hi) = bounds[i];
                let step = (hi - lo) * 0.25;
                xi[i] = (xi[i] + step).clamp(lo, hi);
                if (xi[i] - simplex[0][i]).abs() < 1e-10 {
                    xi[i] = (xi[i] - step * 2.0).clamp(lo, hi);
                }
                simplex.push(xi);
            }

            let mut simplex_loss: Vec<f64> = simplex.iter().map(|x| self.eval_loss_for_params(x)).collect();

            let max_iters = 1000;
            let alpha = 1.0;
            let gamma = 2.0;
            let rho = 0.5;
            let sigma = 0.5;

            for iter in 0..max_iters {
                let mut order: Vec<usize> = (0..simplex.len()).collect();
                order.sort_by(|&a, &b| simplex_loss[a].partial_cmp(&simplex_loss[b]).unwrap());

                let best = order[0];
                let worst = order[n];
                let second_worst = order[n - 1];

                if iter % 200 == 0 {
                    eprintln!("  Iter {}: best={:.6}", iter, simplex_loss[best]);
                }

                let mean_loss = simplex_loss.iter().sum::<f64>() / simplex_loss.len() as f64;
                let std_loss = (simplex_loss.iter()
                    .map(|l| (l - mean_loss).powi(2))
                    .sum::<f64>() / simplex_loss.len() as f64).sqrt();
                if std_loss < 1e-6 {
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
                    reflected[i] = centroid[i] + alpha * (centroid[i] - simplex[worst][i]);
                }
                clamp(&mut reflected);
                let reflected_loss = self.eval_loss_for_params(&reflected);

                if reflected_loss < simplex_loss[second_worst] && reflected_loss >= simplex_loss[best] {
                    simplex[worst] = reflected;
                    simplex_loss[worst] = reflected_loss;
                } else if reflected_loss < simplex_loss[best] {
                    let mut expanded = centroid.clone();
                    for i in 0..n {
                        expanded[i] = centroid[i] + gamma * (reflected[i] - centroid[i]);
                    }
                    clamp(&mut expanded);
                    let expanded_loss = self.eval_loss_for_params(&expanded);

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
                        contracted[i] = centroid[i] + rho * (simplex[worst][i] - centroid[i]);
                    }
                    clamp(&mut contracted);
                    let contracted_loss = self.eval_loss_for_params(&contracted);

                    if contracted_loss < simplex_loss[worst] {
                        simplex[worst] = contracted;
                        simplex_loss[worst] = contracted_loss;
                    } else {
                        for &idx in &order[1..] {
                            for i in 0..n {
                                simplex[idx][i] = simplex[best][i] + sigma * (simplex[idx][i] - simplex[best][i]);
                            }
                            clamp(&mut simplex[idx]);
                            simplex_loss[idx] = self.eval_loss_for_params(&simplex[idx]);
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

            eprintln!("Restart {} best loss: {:.6}", restart, restart_best_loss);
            if restart_best_loss < best_loss {
                best_loss = restart_best_loss;
                best_x = simplex[best_idx].clone();
            }
        }

        eprintln!("Final loss: {:.6}", best_loss);

        Self::apply_params(&mut self.filters, &best_x);
        Ok(())
    }

    fn eval_loss_for_params(&self, x: &[f64]) -> f64 {
        // Apply parameters to a temporary filter clone
        let mut temp_filters: Vec<Box<dyn PeqFilter>> = self.filters.iter()
            .map(|f| f.clone_box())
            .collect();
        Self::apply_params_to_filters_impl(&mut temp_filters, x);

        // Calculate loss
        let n = self.f.len();

        // Compute frequency response
        let mut fr = vec![0.0; n];
        let ln10 = std::f64::consts::LN_10;
        for filter in &temp_filters {
            let (_, a1, a2, b0, b1, b2) = filter.biquad_coefficients();
            let a1n = -a1;
            let a2n = -a2;
            let sum_b = b0 + b1 + b2;
            let sum_a = 1.0 + a1n + a2n;
            let num_base = sum_b * sum_b;
            let den_base = sum_a * sum_a;

            for i in 0..n {
                let phi = self.phi[i];
                let num = num_base + (b0 * b2 * phi - (b1 * (b0 + b2) + 4.0 * b0 * b2)) * phi;
                let den = den_base + (a2n * phi - (a1n * (1.0 + a2n) + 4.0 * a2n)) * phi;
                fr[i] += 10.0 * num.max(1e-30).ln() / ln10 - 10.0 * den.max(1e-30).ln() / ln10;
            }
        }

        // Above 10kHz: replace with mean
        let ix10k = self.f.iter().enumerate()
            .min_by(|(_, a), (_, b)| (*a - 10000.0).abs().partial_cmp(&(*b - 10000.0).abs()).unwrap())
            .map(|(i, _)| i).unwrap_or(n - 1);

        let mut target_adj = self.target.clone();
        let mut fr_adj = fr.clone();

        if ix10k < n {
            let target_mean: f64 = target_adj[ix10k..].iter().sum::<f64>() / (n - ix10k) as f64;
            let fr_mean: f64 = fr_adj[ix10k..].iter().sum::<f64>() / (n - ix10k) as f64;
            for i in ix10k..n {
                target_adj[i] = target_mean;
                fr_adj[i] = fr_mean;
            }
        }

        // MSE in optimization range
        let lo = self.min_f_ix;
        let hi = self.max_f_ix.min(n);

        let mse: f64 = target_adj[lo..hi].iter().zip(fr_adj[lo..hi].iter())
            .map(|(t, f)| (t - f).powi(2))
            .sum::<f64>() / (hi - lo) as f64;

        let penalty: f64 = temp_filters.iter().map(|f| f.sharpness_penalty()).sum();

        (mse + penalty).max(0.0).sqrt()
    }

    fn apply_params(filters: &mut [Box<dyn PeqFilter>], x: &[f64]) {
        Self::apply_params_to_filters_impl(filters, x);
    }

    fn apply_params_to_filters_impl(
        filters: &mut [Box<dyn PeqFilter>],
        x: &[f64],
    ) {
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

/// Result of PEQ optimization
#[derive(Debug, Clone, serde::Serialize)]
pub struct PeqResult {
    pub preamp: f64,
    pub filters: Vec<FilterResult>,
}

#[derive(Debug, Clone, serde::Serialize)]
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
        // At fc, gain should be close to 3 dB
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
        // At low frequencies, gain should be positive
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
        // At high frequencies, gain should be positive
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
}
