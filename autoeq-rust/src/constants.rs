use std::collections::HashMap;
use std::sync::LazyLock;

// --- Global defaults ---
pub const DEFAULT_F_MIN: f64 = 20.0;
pub const DEFAULT_F_MAX: f64 = 20000.0;
pub const DEFAULT_STEP: f64 = 1.01;

pub const DEFAULT_MAX_GAIN: f64 = 6.0;
pub const DEFAULT_TREBLE_F_LOWER: f64 = 6000.0;
pub const DEFAULT_TREBLE_F_UPPER: f64 = 8000.0;
pub const DEFAULT_TREBLE_MAX_GAIN: f64 = 6.0;
pub const DEFAULT_TREBLE_GAIN_K: f64 = 1.0;

pub const DEFAULT_SMOOTHING_WINDOW_SIZE: f64 = 1.0 / 12.0;
pub const DEFAULT_TREBLE_SMOOTHING_F_LOWER: f64 = 6000.0;
pub const DEFAULT_TREBLE_SMOOTHING_F_UPPER: f64 = 8000.0;
pub const DEFAULT_TREBLE_SMOOTHING_WINDOW_SIZE: f64 = 2.0;

pub const DEFAULT_FS: f64 = 44100.0;
pub const DEFAULT_BIT_DEPTH: u32 = 16;
pub const DEFAULT_F_RES: f64 = 10.0;

pub const DEFAULT_TILT: f64 = 0.0;
pub const DEFAULT_BASS_BOOST_GAIN: f64 = 0.0;
pub const DEFAULT_BASS_BOOST_FC: f64 = 105.0;
pub const DEFAULT_BASS_BOOST_Q: f64 = 0.7;
pub const DEFAULT_TREBLE_BOOST_GAIN: f64 = 0.0;
pub const DEFAULT_TREBLE_BOOST_FC: f64 = 10000.0;
pub const DEFAULT_TREBLE_BOOST_Q: f64 = 0.7;

pub const DEFAULT_PEQ_OPTIMIZER_MIN_F: f64 = 20.0;
pub const DEFAULT_PEQ_OPTIMIZER_MAX_F: f64 = 20000.0;
pub const DEFAULT_PEQ_OPTIMIZER_MIN_STD: f64 = 0.002;

pub const DEFAULT_FIXED_BAND_FILTER_MIN_GAIN: f64 = -12.0;
pub const DEFAULT_FIXED_BAND_FILTER_MAX_GAIN: f64 = 12.0;

pub const DEFAULT_PEAKING_FILTER_MIN_FC: f64 = 20.0;
pub const DEFAULT_PEAKING_FILTER_MAX_FC: f64 = 10000.0;
pub const DEFAULT_PEAKING_FILTER_MIN_Q: f64 = 0.18248;
pub const DEFAULT_PEAKING_FILTER_MAX_Q: f64 = 6.0;
pub const DEFAULT_PEAKING_FILTER_MIN_GAIN: f64 = -20.0;
pub const DEFAULT_PEAKING_FILTER_MAX_GAIN: f64 = 20.0;

pub const DEFAULT_SHELF_FILTER_MIN_FC: f64 = 20.0;
pub const DEFAULT_SHELF_FILTER_MAX_FC: f64 = 10000.0;
pub const DEFAULT_SHELF_FILTER_MIN_Q: f64 = 0.4;
pub const DEFAULT_SHELF_FILTER_MAX_Q: f64 = 0.7;
pub const DEFAULT_SHELF_FILTER_MIN_GAIN: f64 = -20.0;
pub const DEFAULT_SHELF_FILTER_MAX_GAIN: f64 = 20.0;

pub const DEFAULT_BIQUAD_OPTIMIZATION_F_STEP: f64 = 1.02;

pub const DEFAULT_MAX_SLOPE: f64 = 18.0;
pub const DEFAULT_PREAMP: f64 = 0.0;

pub const DEFAULT_GRAPHIC_EQ_STEP: f64 = 1.0563;

pub const PREAMP_HEADROOM: f64 = 0.2;

// --- PEQ config types ---

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum FilterType {
    Peaking,
    LowShelf,
    HighShelf,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct OptimizerConfig {
    pub min_f: f64,
    pub max_f: f64,
    pub max_time: Option<f64>,
    pub target_loss: Option<f64>,
    pub min_change_rate: Option<f64>,
    pub min_std: f64,
}

impl Default for OptimizerConfig {
    fn default() -> Self {
        Self {
            min_f: DEFAULT_PEQ_OPTIMIZER_MIN_F,
            max_f: DEFAULT_PEQ_OPTIMIZER_MAX_F,
            max_time: None,
            target_loss: None,
            min_change_rate: None,
            min_std: DEFAULT_PEQ_OPTIMIZER_MIN_STD,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FilterDefaults {
    pub min_fc: Option<f64>,
    pub max_fc: Option<f64>,
    pub min_q: Option<f64>,
    pub max_q: Option<f64>,
    pub min_gain: Option<f64>,
    pub max_gain: Option<f64>,
    pub q: Option<f64>,
    pub filter_type: Option<FilterType>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FilterConfig {
    pub filter_type: Option<FilterType>,
    pub fc: Option<f64>,
    pub q: Option<f64>,
    pub gain: Option<f64>,
    pub min_fc: Option<f64>,
    pub max_fc: Option<f64>,
    pub min_q: Option<f64>,
    pub max_q: Option<f64>,
    pub min_gain: Option<f64>,
    pub max_gain: Option<f64>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PeqConfig {
    pub optimizer: OptimizerConfig,
    pub filter_defaults: Option<FilterDefaults>,
    pub filters: Vec<FilterConfig>,
}

fn peaking_filter() -> FilterConfig {
    FilterConfig {
        filter_type: Some(FilterType::Peaking),
        fc: None, q: None, gain: None,
        min_fc: None, max_fc: None, min_q: None, max_q: None,
        min_gain: None, max_gain: None,
    }
}

fn peaking_filter_with(fc: f64, q: f64) -> FilterConfig {
    FilterConfig {
        filter_type: Some(FilterType::Peaking),
        fc: Some(fc), q: Some(q), gain: None,
        min_fc: None, max_fc: None, min_q: None, max_q: None,
        min_gain: None, max_gain: None,
    }
}

fn low_shelf(fc: f64, q: f64) -> FilterConfig {
    FilterConfig {
        filter_type: Some(FilterType::LowShelf),
        fc: Some(fc), q: Some(q), gain: None,
        min_fc: None, max_fc: None, min_q: None, max_q: None,
        min_gain: None, max_gain: None,
    }
}

fn high_shelf(fc: f64, q: f64) -> FilterConfig {
    FilterConfig {
        filter_type: Some(FilterType::HighShelf),
        fc: Some(fc), q: Some(q), gain: None,
        min_fc: None, max_fc: None, min_q: None, max_q: None,
        min_gain: None, max_gain: None,
    }
}

fn peaking_with_bounds(min_fc: f64, max_fc: f64, min_q: f64, max_q: f64) -> FilterConfig {
    FilterConfig {
        filter_type: Some(FilterType::Peaking),
        fc: None, q: None, gain: None,
        min_fc: Some(min_fc), max_fc: Some(max_fc),
        min_q: Some(min_q), max_q: Some(max_q),
        min_gain: None, max_gain: None,
    }
}

fn peaking_with_full_bounds(min_fc: f64, max_fc: f64, min_q: f64, max_q: f64, min_gain: f64, max_gain: f64) -> FilterConfig {
    FilterConfig {
        filter_type: Some(FilterType::Peaking),
        fc: None, q: None, gain: None,
        min_fc: Some(min_fc), max_fc: Some(max_fc),
        min_q: Some(min_q), max_q: Some(max_q),
        min_gain: Some(min_gain), max_gain: Some(max_gain),
    }
}

pub static PEQ_CONFIGS: LazyLock<HashMap<&'static str, PeqConfig>> = LazyLock::new(|| {
    let mut m = HashMap::new();

    m.insert("10_BAND_GRAPHIC_EQ", PeqConfig {
        optimizer: OptimizerConfig { min_std: 0.01, ..Default::default() },
        filter_defaults: Some(FilterDefaults {
            min_fc: None, max_fc: None, min_q: None, max_q: None,
            min_gain: Some(-12.0), max_gain: Some(12.0),
            q: Some(2.0_f64.sqrt()), filter_type: Some(FilterType::Peaking),
        }),
        filters: (0..10).map(|i| FilterConfig {
            filter_type: None,
            fc: Some(31.25 * 2.0_f64.powi(i)),
            q: None, gain: None,
            min_fc: None, max_fc: None, min_q: None, max_q: None,
            min_gain: None, max_gain: None,
        }).collect(),
    });

    m.insert("31_BAND_GRAPHIC_EQ", PeqConfig {
        optimizer: OptimizerConfig { min_std: 0.01, ..Default::default() },
        filter_defaults: Some(FilterDefaults {
            min_fc: None, max_fc: None, min_q: None, max_q: None,
            min_gain: Some(-12.0), max_gain: Some(12.0),
            q: Some(4.318473), filter_type: Some(FilterType::Peaking),
        }),
        filters: (0..31).map(|i| FilterConfig {
            filter_type: Some(FilterType::Peaking),
            fc: Some(20.0 * 2.0_f64.powf(i as f64 / 3.0)),
            q: None, gain: None,
            min_fc: None, max_fc: None, min_q: None, max_q: None,
            min_gain: None, max_gain: None,
        }).collect(),
    });

    // 10_PEAKING: 10个自由peaking滤波器，用max_time限制优化时长
    m.insert("10_PEAKING", PeqConfig {
        optimizer: OptimizerConfig { max_time: Some(0.5), ..Default::default() },
        filter_defaults: None,
        filters: vec![peaking_filter(); 10],
    });

    m.insert("8_PEAKING_WITH_SHELVES", PeqConfig {
        optimizer: OptimizerConfig { max_time: Some(0.5), min_std: 0.05, ..Default::default() },
        filter_defaults: None,
        filters: {
            let mut v = vec![low_shelf(105.0, 0.7), high_shelf(10000.0, 0.7)];
            v.extend(vec![peaking_filter(); 8]);
            v
        },
    });

    m.insert("4_PEAKING_WITH_SHELVES", PeqConfig {
        optimizer: OptimizerConfig { min_std: 0.008, ..Default::default() },
        filter_defaults: None,
        filters: {
            let mut v = vec![low_shelf(105.0, 0.7), high_shelf(10000.0, 0.7)];
            v.extend(vec![peaking_filter(); 4]);
            v
        },
    });

    m.insert("4_PEAKING_WITH_LOW_SHELF", PeqConfig {
        optimizer: OptimizerConfig { max_f: 10000.0, ..Default::default() },
        filter_defaults: None,
        filters: {
            let mut v = vec![low_shelf(105.0, 0.7)];
            v.extend(vec![peaking_filter(); 4]);
            v
        },
    });

    m.insert("4_PEAKING_WITH_HIGH_SHELF", PeqConfig {
        optimizer: OptimizerConfig::default(),
        filter_defaults: None,
        filters: {
            let mut v = vec![high_shelf(10000.0, 0.7)];
            v.extend(vec![peaking_filter(); 4]);
            v
        },
    });

    m.insert("AUNBANDEQ", PeqConfig {
        optimizer: OptimizerConfig { min_std: 0.008, ..Default::default() },
        filter_defaults: None,
        filters: {
            let mut v = vec![low_shelf(105.0, 0.7), high_shelf(10000.0, 0.7)];
            v.extend(vec![peaking_with_bounds(20.0, 10000.0, 0.182479, 10.0); 8]);
            v
        },
    });

    m.insert("MINIDSP_2X4HD", PeqConfig {
        optimizer: OptimizerConfig { min_std: 0.008, ..Default::default() },
        filter_defaults: Some(FilterDefaults {
            min_fc: None, max_fc: None, min_q: None, max_q: None,
            min_gain: Some(-16.0), max_gain: Some(16.0),
            q: None, filter_type: None,
        }),
        filters: {
            let mut v = vec![low_shelf(105.0, 0.7), high_shelf(10000.0, 0.7)];
            v.extend(vec![peaking_with_bounds(20.0, 10000.0, 0.5, 6.0); 8]);
            v
        },
    });

    m.insert("MINIDSP_IL_DSP", PeqConfig {
        optimizer: OptimizerConfig { min_std: 0.008, ..Default::default() },
        filter_defaults: Some(FilterDefaults {
            min_fc: None, max_fc: None, min_q: None, max_q: None,
            min_gain: Some(-16.0), max_gain: Some(16.0),
            q: None, filter_type: None,
        }),
        filters: {
            let mut v = vec![low_shelf(105.0, 0.7), high_shelf(10000.0, 0.7)];
            v.extend(vec![peaking_with_bounds(20.0, 10000.0, 0.5, 6.0); 8]);
            v
        },
    });

    m.insert("MOONDROP_FREE_DSP", PeqConfig {
        optimizer: OptimizerConfig { min_std: 0.008, ..Default::default() },
        filter_defaults: None,
        filters: vec![peaking_with_full_bounds(40.0, 10000.0, 0.5, 6.0, -12.0, 3.0); 9],
    });

    m.insert("NEUTRON_MUSIC_PLAYER", PeqConfig {
        optimizer: OptimizerConfig { min_std: 0.008, ..Default::default() },
        filter_defaults: Some(FilterDefaults {
            min_fc: None, max_fc: None, min_q: None, max_q: None,
            min_gain: Some(-12.0), max_gain: Some(12.0),
            q: None, filter_type: None,
        }),
        filters: {
            let mut v = vec![low_shelf(105.0, 0.7), high_shelf(10000.0, 0.7)];
            v.extend(vec![peaking_with_bounds(20.0, 10000.0, 0.1, 5.0); 8]);
            v
        },
    });

    m.insert("POWERAMP_EQUALIZER", PeqConfig {
        optimizer: OptimizerConfig { min_std: 0.008, ..Default::default() },
        filter_defaults: Some(FilterDefaults {
            min_fc: None, max_fc: None, min_q: None, max_q: None,
            min_gain: Some(-15.0), max_gain: Some(15.0),
            q: None, filter_type: None,
        }),
        filters: {
            let mut v = vec![low_shelf(105.0, 0.7), high_shelf(10000.0, 0.7)];
            v.extend(vec![peaking_with_bounds(20.0, 10000.0, 0.1, 12.0); 8]);
            v
        },
    });

    m.insert("QUDELIX_5K", PeqConfig {
        optimizer: OptimizerConfig { min_std: 0.008, ..Default::default() },
        filter_defaults: Some(FilterDefaults {
            min_fc: None, max_fc: None, min_q: None, max_q: None,
            min_gain: Some(-12.0), max_gain: Some(12.0),
            q: None, filter_type: None,
        }),
        filters: {
            let mut v = vec![low_shelf(105.0, 0.7), high_shelf(10000.0, 0.7)];
            v.extend(vec![peaking_with_bounds(20.0, 10000.0, 0.1, 7.0); 8]);
            v
        },
    });

    m.insert("SPOTIFY", PeqConfig {
        optimizer: OptimizerConfig { min_std: 0.01, ..Default::default() },
        filter_defaults: None,
        filters: vec![
            peaking_filter_with(60.0, 1.0),
            peaking_filter_with(150.0, 1.0),
            peaking_filter_with(400.0, 1.0),
            peaking_filter_with(1000.0, 1.0),
            peaking_filter_with(2400.0, 1.0),
            peaking_filter_with(15000.0, 1.0),
        ],
    });

    m.insert("USB_AUDIO_PLAYER_PRO", PeqConfig {
        optimizer: OptimizerConfig { min_std: 0.008, ..Default::default() },
        filter_defaults: Some(FilterDefaults {
            min_fc: None, max_fc: None, min_q: None, max_q: None,
            min_gain: Some(-20.0), max_gain: Some(20.0),
            q: None, filter_type: None,
        }),
        filters: {
            let mut v = vec![low_shelf(105.0, 0.7), high_shelf(10000.0, 0.7)];
            v.extend(vec![peaking_with_bounds(20.0, 10000.0, 0.1, 10.0); 8]);
            v
        },
    });

    m
});

pub static DEFAULT_BASS_BOOST_GAINS: LazyLock<HashMap<&'static str, f64>> = LazyLock::new(|| {
    let mut m = HashMap::new();
    m.insert("AutoEq in-ear", 8.0);
    m.insert("crinacle EARS + 711 Harman over-ear 2018 without bass", 6.0);
    m.insert("Harman in-ear 2019 without bass", 9.5);
    m.insert("Harman over-ear 2018 without bass", 6.0);
    m.insert("HMS II.3 AutoEq in-ear", 8.0);
    m.insert("HMS II.3 Harman in-ear 2019 without bass", 9.5);
    m.insert("HMS II.3 Harman over-ear 2018 without bass", 6.0);
    m.insert("JM-1 with Harman treble filter", 6.5);
    m.insert("LMG 5128 0.6 without bass", 6.0);
    m
});

pub fn get_default_filter_type(filter_type: Option<FilterType>, defaults: &Option<FilterDefaults>) -> FilterType {
    if let Some(ft) = filter_type {
        return ft;
    }
    if let Some(ref d) = defaults {
        if let Some(ft) = d.filter_type {
            return ft;
        }
    }
    FilterType::Peaking
}
