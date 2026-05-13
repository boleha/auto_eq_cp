/// Geometric frequency series: f_min, f_min*step, f_min*step^2, ... <= f_max
pub fn generate_frequencies(f_min: f64, f_max: f64, f_step: f64) -> Vec<f64> {
    let mut f = f_min;
    let mut result = Vec::new();
    while f <= f_max {
        result.push(f);
        f *= f_step;
    }
    result
}

/// dB tilt: steepness * log2(f / center_freq)
/// center = 20 * sqrt(20000/20) = 20 * sqrt(1000) ≈ 632.456
pub fn log_tilt(f: &[f64], steepness: f64) -> Vec<f64> {
    let c = 20.0 * (20000.0_f64 / 20.0).sqrt();
    f.iter().map(|&fi| (fi / c).log2() * steepness).collect()
}

/// Octave-based smoothing window to integer sample count (odd, >= 3)
pub fn smoothing_window_size(f: &[f64], octaves: f64) -> usize {
    let k = 2.0_f64.powf(octaves);
    // Average frequency step ratio
    let step_size: f64 = if f.len() < 2 {
        1.01
    } else {
        let sum: f64 = f.windows(2).map(|w| w[1] / w[0]).sum();
        sum / (f.len() - 1) as f64
    };
    let n = (k.ln() / step_size.ln()).round() as usize;
    let n = if n < 3 { 3 } else { n };
    // Make odd
    if n % 2 == 0 { n + 1 } else { n }
}

/// Logistic sigmoid: 1 / (1 + exp(-x))
fn expit(x: f64) -> f64 {
    1.0 / (1.0 + (-x).exp())
}

/// Sigmoid transition in log10(frequency) space between two amplitude levels
pub fn log_f_sigmoid(f: &[f64], f_lower: f64, f_upper: f64, a_normal: f64, a_treble: f64) -> Vec<f64> {
    let f_center = (f_upper / f_lower).sqrt() * f_lower;
    let half_range = f_upper.log10() - f_center.log10();
    let f_center_log = f_center.log10();
    let denom = half_range / 4.0;

    f.iter().map(|&fi| {
        let a = expit((fi.log10() - f_center_log) / denom);
        a * -(a_normal - a_treble) + a_normal
    }).collect()
}

/// dB per octave gradient between two frequency/gain points
pub fn log_log_gradient(f0: f64, f1: f64, g0: f64, g1: f64) -> f64 {
    let octaves = (f1 / f0).log2();
    (g1 - g0) / octaves
}

/// Next fast FFT length (product of small primes 2, 3, 5, 7)
pub fn next_fast_len(n: usize) -> usize {
    if n <= 1 { return 1; }
    let mut best = n;
    // Try powers of 2, 3, 5, 7 combinations
    // Simple approach: find next number that factors completely into 2,3,5,7
    loop {
        if is_fast_len(best) {
            return best;
        }
        best += 1;
    }
}

fn is_fast_len(mut n: usize) -> bool {
    if n == 0 { return false; }
    for &p in &[2, 3, 5, 7] {
        while n % p == 0 {
            n /= p;
        }
    }
    n == 1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_frequencies() {
        let f = generate_frequencies(20.0, 20000.0, 1.01);
        assert!(f.len() > 600);
        assert!((f[0] - 20.0).abs() < 1e-10);
        // Last frequency should be <= 20000
        assert!(f[f.len() - 1] <= 20000.0 + 1e-6);
    }

    #[test]
    fn test_log_tilt() {
        let f = vec![632.4555320336759]; // center frequency
        let tilt = log_tilt(&f, 1.0);
        assert!((tilt[0]).abs() < 1e-10); // should be ~0 at center
    }

    #[test]
    fn test_smoothing_window_size() {
        let f = generate_frequencies(20.0, 20000.0, 1.01);
        let n = smoothing_window_size(&f, 1.0 / 12.0);
        assert!(n >= 3);
        assert!(n % 2 == 1);
    }

    #[test]
    fn test_log_f_sigmoid() {
        let f = vec![6000.0, 7000.0, 8000.0];
        let s = log_f_sigmoid(&f, 6000.0, 8000.0, 0.0, 1.0);
        // At f_lower: should be close to 0
        assert!(s[0] < 0.1);
        // At f_upper: should be close to 1
        assert!(s[2] > 0.9);
    }

    #[test]
    fn test_log_log_gradient() {
        // 6 dB over 1 octave
        let g = log_log_gradient(100.0, 200.0, 0.0, 6.0);
        assert!((g - 6.0).abs() < 1e-10);
    }

    #[test]
    fn test_next_fast_len() {
        assert_eq!(next_fast_len(1), 1);
        assert_eq!(next_fast_len(2), 2);
        assert_eq!(next_fast_len(3), 3);
        assert!(next_fast_len(100) >= 100);
        assert!(is_fast_len(next_fast_len(100)));
    }
}
