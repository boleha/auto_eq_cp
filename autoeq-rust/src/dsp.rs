use crate::utils::next_fast_len;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// 全局 FFT 变换缓存，按 n_fft 大小复用，避免每次重新计算 twiddle factors
/// realfft::RealFftPlanner::plan_* 返回的 Arc<dyn *> 可以安全缓存和复用
static FFT_CACHE: Mutex<Option<FftCache>> = Mutex::new(None);

struct FftCache {
    forward: HashMap<usize, Arc<dyn realfft::RealToComplex<f64>>>,
    inverse: HashMap<usize, Arc<dyn realfft::ComplexToReal<f64>>>,
}

fn get_fft_forward(n_fft: usize) -> Arc<dyn realfft::RealToComplex<f64>> {
    let mut cache_opt = FFT_CACHE.lock().unwrap();
    if cache_opt.is_none() {
        *cache_opt = Some(FftCache {
            forward: HashMap::new(),
            inverse: HashMap::new(),
        });
    }
    let cache = cache_opt.as_mut().unwrap();
    if let Some(plan) = cache.forward.get(&n_fft) {
        return Arc::clone(plan);
    }
    let mut planner = realfft::RealFftPlanner::<f64>::new();
    let plan = planner.plan_fft_forward(n_fft);
    cache.forward.insert(n_fft, Arc::clone(&plan));
    plan
}

fn get_fft_inverse(n_fft: usize) -> Arc<dyn realfft::ComplexToReal<f64>> {
    let mut cache_opt = FFT_CACHE.lock().unwrap();
    if cache_opt.is_none() {
        *cache_opt = Some(FftCache {
            forward: HashMap::new(),
            inverse: HashMap::new(),
        });
    }
    let cache = cache_opt.as_mut().unwrap();
    if let Some(plan) = cache.inverse.get(&n_fft) {
        return Arc::clone(plan);
    }
    let mut planner = realfft::RealFftPlanner::<f64>::new();
    let plan = planner.plan_fft_inverse(n_fft);
    cache.inverse.insert(n_fft, Arc::clone(&plan));
    plan
}

/// Savitzky-Golay smoothing filter.
/// window_length: must be odd and >= 3
/// polyorder: polynomial order (2 in this codebase)
/// Returns smoothed data of same length as input.
pub fn savgol_filter(data: &[f64], window_length: usize, polyorder: usize) -> Vec<f64> {
    assert!(window_length % 2 == 1, "window_length must be odd");
    assert!(window_length >= 3, "window_length must be >= 3");
    assert!(polyorder < window_length, "polyorder must be less than window_length");

    let half = window_length / 2;
    let coeffs = savgol_coeffs(window_length, polyorder);

    // Convolve with edge handling (extend with boundary values)
    let mut result = vec![0.0; data.len()];
    for i in 0..data.len() {
        let mut sum = 0.0;
        for (j, &c) in coeffs.iter().enumerate() {
            let idx = i as isize + j as isize - half as isize;
            let idx = idx.max(0).min(data.len() as isize - 1) as usize;
            sum += c * data[idx];
        }
        result[i] = sum;
    }
    result
}

/// Compute Savitzky-Golay convolution coefficients.
/// Returns the first row of (J^T J)^{-1} J^T, which are the smoothing weights.
fn savgol_coeffs(window_length: usize, polyorder: usize) -> Vec<f64> {
    let half = window_length / 2;
    let m = polyorder + 1;

    // Build J: window_length x m matrix
    // J[i][j] = (i - half)^j
    let mut jtm = vec![vec![0.0; m]; window_length]; // J^T (stored as rows)
    for i in 0..window_length {
        let x = i as f64 - half as f64;
        let mut val = 1.0;
        for j in 0..m {
            jtm[i][j] = val;
            val *= x;
        }
    }

    // Compute J^T * J: m x m matrix
    let mut jtj = vec![vec![0.0; m]; m];
    for i in 0..m {
        for j in 0..m {
            let mut sum = 0.0;
            for k in 0..window_length {
                sum += jtm[k][i] * jtm[k][j];
            }
            jtj[i][j] = sum;
        }
    }

    // Invert J^T * J (small matrix, use Gauss-Jordan)
    let jtj_inv = mat_inv(&jtj, m);

    // Compute (J^T J)^{-1} * J^T: m x window_length matrix
    // We only need the first row (row 0) for the 0th derivative
    let mut coeffs = vec![0.0; window_length];
    for k in 0..window_length {
        let mut sum = 0.0;
        for j in 0..m {
            sum += jtj_inv[0][j] * jtm[k][j];
        }
        coeffs[k] = sum;
    }

    coeffs
}

/// Matrix inversion via Gauss-Jordan elimination
fn mat_inv(mat: &[Vec<f64>], n: usize) -> Vec<Vec<f64>> {
    // Augment with identity
    let mut aug = vec![vec![0.0; 2 * n]; n];
    for i in 0..n {
        for j in 0..n {
            aug[i][j] = mat[i][j];
        }
        aug[i][n + i] = 1.0;
    }

    for col in 0..n {
        // Find pivot
        let mut max_val = aug[col][col].abs();
        let mut max_row = col;
        for row in (col + 1)..n {
            if aug[row][col].abs() > max_val {
                max_val = aug[row][col].abs();
                max_row = row;
            }
        }
        // Swap rows
        aug.swap(col, max_row);

        let pivot = aug[col][col];
        assert!(pivot.abs() > 1e-15, "Singular matrix in savgol_coeffs");

        // Scale pivot row
        for j in 0..(2 * n) {
            aug[col][j] /= pivot;
        }

        // Eliminate column
        for row in 0..n {
            if row == col { continue; }
            let factor = aug[row][col];
            for j in 0..(2 * n) {
                aug[row][j] -= factor * aug[col][j];
            }
        }
    }

    // Extract inverse
    let mut inv = vec![vec![0.0; n]; n];
    for i in 0..n {
        for j in 0..n {
            inv[i][j] = aug[i][n + j];
        }
    }
    inv
}

/// Find peaks in data with prominence filtering.
/// Returns (peak_indices, prominences, widths, peak_heights)
pub fn find_peaks_with_props(data: &[f64], min_prominence: f64) -> (Vec<usize>, Vec<f64>, Vec<f64>, Vec<f64>) {
    if data.len() < 3 {
        return (vec![], vec![], vec![], vec![]);
    }

    // Step 1: Find all local maxima
    let mut peak_inds = Vec::new();
    for i in 1..(data.len() - 1) {
        if data[i] > data[i - 1] && data[i] > data[i + 1] {
            peak_inds.push(i);
        }
    }

    if peak_inds.is_empty() {
        return (vec![], vec![], vec![], vec![]);
    }

    // Step 2: Compute prominence for each peak
    let peak_heights: Vec<f64> = peak_inds.iter().map(|&i| data[i]).collect();
    let prominences = compute_prominences(data, &peak_inds, &peak_heights);

    // Step 3: Filter by prominence
    let mut filtered_inds = Vec::new();
    let mut filtered_prom = Vec::new();
    let mut filtered_heights = Vec::new();
    for (i, &p) in prominences.iter().enumerate() {
        if p >= min_prominence {
            filtered_inds.push(peak_inds[i]);
            filtered_prom.push(p);
            filtered_heights.push(peak_heights[i]);
        }
    }

    // Step 4: Compute widths at half-prominence
    let widths = compute_widths(data, &filtered_inds, &filtered_heights, &filtered_prom);

    (filtered_inds, filtered_prom, widths, filtered_heights)
}

/// Find peaks (simple, without prominence - just local maxima)
pub fn find_peaks(data: &[f64]) -> Vec<usize> {
    if data.len() < 3 {
        return vec![];
    }
    let mut peaks = Vec::new();
    for i in 1..(data.len() - 1) {
        if data[i] > data[i - 1] && data[i] > data[i + 1] {
            peaks.push(i);
        }
    }
    peaks
}

/// Find dips (local minima) by finding peaks in negated data
pub fn find_dips(data: &[f64]) -> Vec<usize> {
    let neg: Vec<f64> = data.iter().map(|&x| -x).collect();
    find_peaks(&neg)
}

fn compute_prominences(data: &[f64], peak_inds: &[usize], peak_heights: &[f64]) -> Vec<f64> {
    let n_peaks = peak_inds.len();
    let mut prominences = vec![0.0; n_peaks];

    for i in 0..n_peaks {
        let peak_pos = peak_inds[i];
        let peak_h = peak_heights[i];

        // Walk left to find the minimum value between this peak and the first higher peak to the left
        let mut left_min = peak_h;
        let mut found_higher_left = false;
        for j in (0..peak_pos).rev() {
            left_min = left_min.min(data[j]);
            for k in 0..n_peaks {
                if peak_inds[k] < peak_pos && peak_inds[k] >= j && peak_heights[k] > peak_h {
                    found_higher_left = true;
                    break;
                }
            }
            if found_higher_left { break; }
        }

        // Walk right
        let mut right_min = peak_h;
        let mut found_higher_right = false;
        for j in (peak_pos + 1)..data.len() {
            right_min = right_min.min(data[j]);
            for k in 0..n_peaks {
                if peak_inds[k] > peak_pos && peak_inds[k] <= j && peak_heights[k] > peak_h {
                    found_higher_right = true;
                    break;
                }
            }
            if found_higher_right { break; }
        }

        let key_col = left_min.max(right_min);
        prominences[i] = (peak_h - key_col).max(0.0);
    }

    prominences
}

fn compute_widths(data: &[f64], peak_inds: &[usize], peak_heights: &[f64], prominences: &[f64]) -> Vec<f64> {
    let mut widths = vec![0.0; peak_inds.len()];

    for i in 0..peak_inds.len() {
        let pos = peak_inds[i];
        let h = peak_heights[i];
        let half_height = h - prominences[i] / 2.0;

        // Walk left from peak to find where data drops below half_height
        let mut left = pos;
        for j in (0..pos).rev() {
            if data[j] < half_height {
                left = j;
                break;
            }
            if j == 0 { left = 0; break; }
        }

        // Walk right
        let mut right = pos;
        for j in (pos + 1)..data.len() {
            if data[j] < half_height {
                right = j;
                break;
            }
            if j == data.len() - 1 { right = data.len() - 1; break; }
        }

        widths[i] = (right as f64) - (left as f64);
    }

    widths
}

/// Design FIR filter with arbitrary frequency response using frequency sampling method.
/// num_taps: filter length
/// freq: frequency breakpoints (Hz), must start at 0 and end at fs/2
/// gain: gain at each breakpoint (linear scale, not dB)
/// fs: sampling frequency
pub fn firwin2(num_taps: usize, freq: &[f64], gain: &[f64], fs: f64) -> Vec<f64> {
    assert_eq!(freq.len(), gain.len(), "freq and gain must have same length");
    assert!(freq.len() >= 2, "need at least 2 frequency points");

    let n_fft = if num_taps % 2 == 0 { num_taps } else { num_taps + 1 };
    let df = fs / n_fft as f64;
    let n_freq = n_fft / 2 + 1;

    // Interpolate gain at grid points
    let mut gain_interp = vec![0.0; n_freq];
    for i in 0..n_freq {
        let f = i as f64 * df;
        gain_interp[i] = linear_interp(f, freq, gain);
    }

    // Build complex spectrum with linear phase
    let mut spectrum = vec![num_complex::Complex::new(0.0, 0.0); n_freq];
    for i in 0..n_freq {
        let phase = -std::f64::consts::PI * i as f64 * (num_taps as f64 - 1.0) / n_fft as f64;
        spectrum[i] = num_complex::Complex::new(gain_interp[i] * phase.cos(), gain_interp[i] * phase.sin());
    }

    // IFFT using ComplexToReal (使用缓存的 FFT plan)
    let c2r = get_fft_inverse(n_fft);
    let mut output_r = vec![0.0; n_fft];
    c2r.process(&mut spectrum, &mut output_r).unwrap();

    // Normalize and truncate to num_taps
    let scale = 1.0 / n_fft as f64;
    let mut result = vec![0.0; num_taps];
    for i in 0..num_taps {
        result[i] = output_r[i] * scale;
    }

    result
}

/// Convert linear-phase FIR to minimum-phase FIR using homomorphic method.
pub fn minimum_phase(linear_ir: &[f64]) -> Vec<f64> {
    let n = linear_ir.len();
    let n_fft = next_fast_len(n * 2).max(64);

    // Use pure complex FFT for the homomorphic method (使用缓存的 FFT plan)
    let r2c = get_fft_forward(n_fft);
    let c2r = get_fft_inverse(n_fft);

    // Step 1: FFT of zero-padded linear IR
    let mut padded = vec![0.0; n_fft];
    padded[..n].copy_from_slice(linear_ir);
    let mut spectrum = r2c.make_output_vec();
    r2c.process(&mut padded, &mut spectrum).unwrap();

    // Step 2: Log magnitude spectrum (real-valued)
    let epsilon = 1e-10_f64;
    let mut log_mag = vec![0.0; n_fft];
    log_mag[0] = (spectrum[0].norm() + epsilon).ln();
    for i in 1..(n_fft / 2) {
        let m = (spectrum[i].norm() + epsilon).ln();
        log_mag[i] = m;
        log_mag[n_fft - i] = m;
    }
    log_mag[n_fft / 2] = (spectrum[n_fft / 2].norm() + epsilon).ln();

    // Step 3: IFFT of log magnitude -> real cepstrum
    // Use C2R with full complex spectrum
    let mut log_mag_full: Vec<num_complex::Complex<f64>> = Vec::with_capacity(n_fft / 2 + 1);
    for i in 0..(n_fft / 2 + 1) {
        log_mag_full.push(num_complex::Complex::new(log_mag[i], 0.0));
    }
    let mut cepstrum = vec![0.0; n_fft];
    c2r.process(&mut log_mag_full, &mut cepstrum).unwrap();
    let scale = 1.0 / n_fft as f64;
    for x in cepstrum.iter_mut() { *x *= scale; }

    // Step 4: Lifter - double positive quefrencies, keep zero, zero negative
    let mut liftered = vec![0.0; n_fft];
    liftered[0] = cepstrum[0];
    for i in 1..(n_fft / 2) {
        liftered[i] = 2.0 * cepstrum[i];
    }
    if n_fft % 2 == 0 {
        liftered[n_fft / 2] = cepstrum[n_fft / 2];
    }

    // Step 5: FFT of liftered cepstrum -> log min-phase spectrum
    let mut liftered_spectrum = r2c.make_output_vec();
    r2c.process(&mut liftered, &mut liftered_spectrum).unwrap();
    let scale = 1.0 / n_fft as f64;
    for c in liftered_spectrum.iter_mut() { *c *= scale; }

    // Step 6: Exponentiate to get minimum-phase spectrum
    let mut min_phase_spectrum = Vec::with_capacity(n_fft / 2 + 1);
    for i in 0..(n_fft / 2 + 1) {
        let re = liftered_spectrum[i].re;
        let im = liftered_spectrum[i].im;
        let exp_val = re.exp();
        min_phase_spectrum.push(num_complex::Complex::new(exp_val * im.cos(), exp_val * im.sin()));
    }

    // Step 7: IFFT to get minimum-phase IR
    let mut min_phase_ir = vec![0.0; n_fft];
    c2r.process(&mut min_phase_spectrum, &mut min_phase_ir).unwrap();
    let scale = 1.0 / n_fft as f64;

    let mut result = vec![0.0; n];
    result[..n].copy_from_slice(&min_phase_ir[..n]);
    for x in result.iter_mut() { *x *= scale; }

    result
}

/// Linear interpolation helper
fn linear_interp(x: f64, xp: &[f64], fp: &[f64]) -> f64 {
    if x <= xp[0] { return fp[0]; }
    if x >= xp[xp.len() - 1] { return fp[fp.len() - 1]; }

    // Binary search for interval
    let mut lo = 0;
    let mut hi = xp.len() - 1;
    while lo < hi - 1 {
        let mid = (lo + hi) / 2;
        if xp[mid] <= x {
            lo = mid;
        } else {
            hi = mid;
        }
    }

    let t = (x - xp[lo]) / (xp[hi] - xp[lo]);
    fp[lo] + t * (fp[hi] - fp[lo])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_savgol_filter_identity() {
        // With polyorder=1 and window=3 on linear data, interior points should be preserved
        // (edges differ due to boundary extension)
        let data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let result = savgol_filter(&data, 3, 1);
        for i in 1..(data.len() - 1) {
            assert!((result[i] - data[i]).abs() < 1e-10, "i={}: {} vs {}", i, result[i], data[i]);
        }
    }

    #[test]
    fn test_savgol_filter_smoothing() {
        // Noisy data should be smoothed
        let data = vec![0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0];
        let result = savgol_filter(&data, 5, 2);
        // Middle values should be smoothed
        assert!(result[3] > 0.0 && result[3] < 1.0);
    }

    #[test]
    fn test_find_peaks() {
        let data = vec![0.0, 1.0, 0.0, 2.0, 0.0, 3.0, 0.0];
        let peaks = find_peaks(&data);
        assert_eq!(peaks, vec![1, 3, 5]);
    }

    #[test]
    fn test_find_peaks_with_prominence() {
        let mut data = vec![0.0; 100];
        // Big peak at 50
        for i in 40..60 {
            data[i] = 5.0 - ((i as f64 - 50.0).abs() * 0.5);
        }
        // Small peak at 20
        for i in 15..25 {
            data[i] = 0.5 - ((i as f64 - 20.0).abs() * 0.05);
        }

        let (peaks, proms, _widths, _heights) = find_peaks_with_props(&data, 1.0);
        // Only the big peak should pass prominence filter
        assert!(peaks.contains(&50));
        // Small peak might not pass (depends on exact prominence)
    }

    #[test]
    fn test_find_dips() {
        let data = vec![3.0, 1.0, 3.0, 0.0, 3.0];
        let dips = find_dips(&data);
        assert_eq!(dips, vec![1, 3]);
    }

    #[test]
    fn test_firwin2_basic() {
        // Design a simple lowpass filter
        let freq = vec![0.0, 1000.0, 1000.0, 22050.0];
        let gain = vec![1.0, 1.0, 0.0, 0.0];
        let taps = firwin2(65, &freq, &gain, 44100.0);
        assert_eq!(taps.len(), 65);
        // The taps should sum to approximately 1.0 (DC gain)
        let sum: f64 = taps.iter().sum();
        assert!((sum - 1.0).abs() < 0.1, "sum = {}", sum);
    }

    #[test]
    fn test_minimum_phase_basic() {
        // Simple linear phase IR
        let linear = vec![0.1, 0.2, 0.3, 0.4, 0.3, 0.2, 0.1];
        let min_phase = minimum_phase(&linear);
        assert_eq!(min_phase.len(), linear.len());
        // Energy should be mostly in the first half
        let early_energy: f64 = min_phase[..4].iter().map(|x| x * x).sum();
        let total_energy: f64 = min_phase.iter().map(|x| x * x).sum();
        assert!(early_energy > total_energy * 0.5, "Early energy should dominate for minimum phase");
    }
}
