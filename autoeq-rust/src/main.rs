use autoeq::api::{self, EqualizeResult, ProcessParams};
use autoeq::peq::PeqResult;
use std::env;
use std::path::Path;
use std::time::Instant;
use autoeq::csv;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage();
        return;
    }

    match args[1].as_str() {
        "equalize" => cmd_equalize(&args[2..]),
        "configs" => cmd_configs(),
        "bench" => cmd_bench(),
        "help" | "--help" | "-h" => print_usage(),
        _ => {
            eprintln!("Unknown command: {}", args[1]);
            print_usage();
        }
    }
}

fn print_usage() {
    println!("AutoEq Rust - Headphone equalization tool");
    println!();
    println!("Usage:");
    println!("  autoeq equalize <input.csv> [--target <target.csv>] [--config <name>] [--name <name>]");
    println!("  autoeq configs");
    println!("  autoeq help");
    println!();
    println!("Examples:");
    println!("  autoeq equalize test_file/harman2016.txt");
    println!("  autoeq equalize measurement.csv --target targets/Harman over-ear 2018.csv --config QUDELIX_5K");
    println!("  autoeq configs");
}

fn cmd_equalize(args: &[String]) {
    if args.is_empty() {
        eprintln!("Error: input file required");
        return;
    }

    let input_path = &args[0];
    let mut target_path: Option<&str> = None;
    let mut config_name = "8_PEAKING_WITH_SHELVES";
    let mut name: Option<&str> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--target" => {
                i += 1;
                if i < args.len() { target_path = Some(&args[i]); }
            }
            "--config" => {
                i += 1;
                if i < args.len() { config_name = &args[i]; }
            }
            "--name" => {
                i += 1;
                if i < args.len() { name = Some(&args[i]); }
            }
            _ => {
                eprintln!("Unknown option: {}", args[i]);
                return;
            }
        }
        i += 1;
    }

    let params = ProcessParams::default();
    let target = target_path.map(Path::new);

    println!("Processing: {}", input_path);
    if let Some(tp) = target_path {
        println!("Target: {}", tp);
    }
    println!("Config: {}", config_name);
    println!();

    match api::equalize_file(Path::new(input_path), target, name, &params, config_name) {
        Ok(result) => {
            print_eq_result(&result.eq_result);
            print_peq_result(&result.parametric_eq);
            println!();
            println!("GraphicEQ:");
            println!("{}", result.graphic_eq_string);
        }
        Err(e) => {
            eprintln!("Error: {}", e);
        }
    }
}

fn cmd_configs() {
    let configs = api::get_available_configs();
    println!("Available PEQ configurations ({}):", configs.len());
    let mut sorted = configs;
    sorted.sort();
    for c in sorted {
        println!("  {}", c);
    }
}

fn print_eq_result(result: &EqualizeResult) {
    println!("Name: {}", result.name);
    println!("Frequency points: {}", result.frequency.len());
    if !result.frequency.is_empty() {
        println!("Frequency range: {:.1} - {:.1} Hz", result.frequency[0], result.frequency[result.frequency.len() - 1]);
    }
    if !result.raw.is_empty() {
        let max_raw = result.raw.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let min_raw = result.raw.iter().cloned().fold(f64::INFINITY, f64::min);
        println!("Raw range: {:.2} - {:.2} dB", min_raw, max_raw);
    }
    if !result.equalization.is_empty() {
        let max_eq = result.equalization.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let min_eq = result.equalization.iter().cloned().fold(f64::INFINITY, f64::min);
        println!("Equalization range: {:.2} - {:.2} dB", min_eq, max_eq);
    }
}

fn print_peq_result(result: &PeqResult) {
    println!();
    println!("Parametric EQ (preamp: {:.2} dB):", result.preamp);
    for (i, f) in result.filters.iter().enumerate() {
        println!("  Filter {}: {:?} fc={:.1} Hz gain={:.2} dB q={:.4}", i + 1, f.filter_type, f.fc, f.gain, f.q);
    }
}

fn cmd_bench() {
    use autoeq::frequency_response::FrequencyResponse;

    let measure_path = "../test_file/harman2016.txt";
    let target_path = "../test_file/5128DF1.5.txt";

    println!("=== AutoEq Rust Benchmark ===");
    println!();

    let total_start = Instant::now();

    // 用真实测量数据
    let t0 = Instant::now();
    let measure_bytes = std::fs::read(measure_path).expect("read measure");
    let measure_text = String::from_utf8_lossy(&measure_bytes);
    let parsed = csv::parse_csv(&measure_text).expect("parse measure");
    let freq = parsed.frequency.clone();
    let raw = parsed.raw.clone();
    println!("  [{}ms] 解析测量文件: {} 数据点 ({} - {} Hz)", t0.elapsed().as_millis(), freq.len(), freq[0], freq.last().unwrap());

    // 用真实目标数据
    let t0 = Instant::now();
    let target_bytes = std::fs::read(target_path).expect("read target");
    let target_text = String::from_utf8_lossy(&target_bytes);
    let target_parsed = csv::parse_csv(&target_text).expect("parse target");
    let target_freq = target_parsed.frequency.clone();
    let target_raw = target_parsed.raw.clone();
    println!("  [{}ms] 解析目标文件: {} 数据点", t0.elapsed().as_millis(), target_freq.len());

    let params = ProcessParams::default();

    // 关键：target 频率轴与测量不同，先插值对齐
    let mut target_fr = FrequencyResponse::new("target_aligned", target_freq, target_raw).unwrap();
    let _ = target_fr.interpolate(Some(&freq), 1.01, 20.0, 20000.0);
    let target_aligned = target_fr.raw;

    println!();
    println!("--- optimize_parametric_eq (8_PEAKING_WITH_SHELVES) ---");
    let t0 = Instant::now();
    let peq_result = api::optimize_parametric_eq(
        &freq,
        &raw,
        "select",
        &params,
        "8_PEAKING_WITH_SHELVES",
        Some(&target_aligned),
    );
    match peq_result {
        Ok(r) => {
            println!("  [{}ms] 完成, {} 个滤波器, preamp={:.2}", t0.elapsed().as_millis(), r.filters.len(), r.preamp);
            for (i, f) in r.filters.iter().enumerate() {
                println!("    F{}: {:?} fc={:.1}Hz q={:.4} gain={:.2}dB", i+1, f.filter_type, f.fc, f.q, f.gain);
            }
        },
        Err(e) => println!("  [{}ms] 失败: {}", t0.elapsed().as_millis(), e),
    }

    println!();
    println!("--- optimize_parametric_eq (10_PEAKING) ---");
    let t0 = Instant::now();
    let peq_result = api::optimize_parametric_eq(
        &freq,
        &raw,
        "select",
        &params,
        "10_PEAKING",
        Some(&target_aligned),
    );
    match peq_result {
        Ok(r) => {
            println!("  [{}ms] 完成, {} 个滤波器, preamp={:.2}", t0.elapsed().as_millis(), r.filters.len(), r.preamp);
            for (i, f) in r.filters.iter().enumerate() {
                println!("    F{}: {:?} fc={:.1}Hz q={:.4} gain={:.2}dB", i+1, f.filter_type, f.fc, f.q, f.gain);
            }
        },
        Err(e) => println!("  [{}ms] 失败: {}", t0.elapsed().as_millis(), e),
    }

    println!();
    println!("=== 总耗时: {}ms ===", total_start.elapsed().as_millis());
}
