use crate::error::{AutoEqError, Result};
use regex::Regex;
use std::collections::BTreeMap;
use std::sync::LazyLock;

/// Parsed CSV result containing frequency and raw (SPL) data
pub struct ParsedCsv {
    pub frequency: Vec<f64>,
    pub raw: Vec<f64>,
}

/// Known AutoEq column names
const AUTOEQ_COLUMNS: &[&str] = &[
    "raw", "smoothed", "error", "error_smoothed", "equalization",
    "parametric_eq", "fixed_band_eq", "equalized_raw", "equalized_smoothed", "target",
];

// Compiled regex patterns for format detection
static AUTOEQ_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    let cols = AUTOEQ_COLUMNS.join("|");
    Regex::new(&format!(r"(?i)^frequency(?:,(?:{}))+$", cols)).unwrap()
});

static REW_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^\*.*Freq\(Hz\).*SPL\(dB\)").unwrap()
});

static CRINACLE_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^Frequency\tdB\tUnweighted").unwrap()
});

static FREQ_HEADER_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^freq").unwrap()
});

static RAW_HEADER_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^(?:spl|gain|ampl|raw)").unwrap()
});

static NUMERIC_START: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\d").unwrap()
});

/// Parse CSV text and extract frequency + raw (SPL) data.
/// Supports AutoEq, REW, Crinacle, and generic CSV formats.
pub fn parse_csv(csv: &str) -> Result<ParsedCsv> {
    // Clean up: strip, remove blank lines
    let cleaned: String = csv.lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("\n");

    // Try AutoEq format first
    if let Some(first_line) = cleaned.lines().next() {
        if AUTOEQ_PATTERN.is_match(first_line) {
            return parse_autoeq(&cleaned);
        }
    }

    // Try REW format
    if REW_PATTERN.is_match(&cleaned) {
        return parse_rew(&cleaned);
    }

    // Try Crinacle format
    if CRINACLE_PATTERN.is_match(&cleaned) {
        return parse_crinacle(&cleaned);
    }

    // Generic CSV
    parse_generic(&cleaned)
}

/// Parse AutoEq native CSV format: header line "frequency,col,col,..." followed by comma-separated floats
fn parse_autoeq(csv: &str) -> Result<ParsedCsv> {
    let mut lines = csv.lines();
    let header = lines.next().ok_or_else(|| AutoEqError::CsvParse("Empty CSV".into()))?;
    let columns: Vec<&str> = header.split(',').map(|s| s.trim()).collect();

    let freq_col = columns.iter().position(|c| c.eq_ignore_ascii_case("frequency"))
        .ok_or_else(|| AutoEqError::CsvParse("No frequency column".into()))?;
    let raw_col = columns.iter().position(|c| c.eq_ignore_ascii_case("raw"))
        .or_else(|| columns.iter().position(|c| c.eq_ignore_ascii_case("spl")))
        .unwrap_or(1);

    let mut frequency = Vec::new();
    let mut raw = Vec::new();

    for line in lines {
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() <= freq_col.max(raw_col) { continue; }
        if let (Ok(f), Ok(r)) = (parts[freq_col].trim().parse::<f64>(), parts[raw_col].trim().parse::<f64>()) {
            frequency.push(f);
            raw.push(r);
        }
    }

    if frequency.is_empty() {
        return Err(AutoEqError::CsvParse("No data found in AutoEq CSV".into()));
    }

    Ok(ParsedCsv { frequency, raw })
}

/// Parse REW (Room EQ Wizard) format: lines starting with * are headers, then space/tab separated data
fn parse_rew(csv: &str) -> Result<ParsedCsv> {
    let mut frequency = Vec::new();
    let mut raw = Vec::new();

    for line in csv.lines() {
        let trimmed = line.trim();
        // Skip comment/header lines
        if trimmed.starts_with('*') || trimmed.is_empty() { continue; }
        // Skip lines with '?' (missing data)
        if trimmed.contains('?') { continue; }
        // Must start with a digit
        if !trimmed.starts_with(|c: char| c.is_ascii_digit()) { continue; }

        // Split by whitespace (handles space, tab, and multiple spaces)
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.len() < 2 { continue; }

        if let (Ok(f), Ok(r)) = (parts[0].parse::<f64>(), parts[1].parse::<f64>()) {
            frequency.push(f);
            raw.push(r);
        }
    }

    if frequency.is_empty() {
        return Err(AutoEqError::CsvParse("No valid data in REW format".into()));
    }

    Ok(ParsedCsv { frequency, raw })
}

/// Parse Crinacle format: "Frequency\tdB\tUnweighted" header, then tab-separated data
fn parse_crinacle(csv: &str) -> Result<ParsedCsv> {
    // Skip until we find the header
    let data_start = csv.lines()
        .position(|l| l.trim().starts_with("Frequency"))
        .ok_or_else(|| AutoEqError::CsvParse("No Crinacle header found".into()))?;

    let mut frequency = Vec::new();
    let mut raw = Vec::new();

    for line in csv.lines().skip(data_start + 1) {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 2 { continue; }
        if let (Ok(f), Ok(r)) = (parts[0].trim().parse::<f64>(), parts[1].trim().parse::<f64>()) {
            frequency.push(f);
            raw.push(r);
        }
    }

    if frequency.is_empty() {
        return Err(AutoEqError::CsvParse("No valid data in Crinacle format".into()));
    }

    Ok(ParsedCsv { frequency, raw })
}

/// Parse generic CSV with auto-detection of separators and column headers
fn parse_generic(csv: &str) -> Result<ParsedCsv> {
    let (col_sep, dec_sep) = find_csv_separators(csv)?;
    let columns = find_csv_columns(csv, col_sep);

    // Identify frequency and raw columns
    let (freq_col_idx, raw_col_idx) = if let Some(ref cols) = columns {
        let mut freq_idx = None;
        let mut raw_idx = None;
        for (i, col) in cols.iter().enumerate() {
            if FREQ_HEADER_RE.is_match(col) {
                freq_idx = Some(i);
            }
            if RAW_HEADER_RE.is_match(col) {
                raw_idx = Some(i);
            }
        }
        // Fallback: if exactly 2 columns, assume freq=0, raw=1
        if freq_idx.is_none() && raw_idx.is_none() && cols.len() == 2 {
            (0, 1)
        } else {
            (freq_idx.unwrap_or(0), raw_idx.unwrap_or(1))
        }
    } else {
        (0, 1)
    };

    // Extract data
    let mut frequency = Vec::new();
    let mut raw = Vec::new();

    for line in csv.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with(|c: char| c.is_ascii_digit()) { continue; }

        let parts: Vec<&str> = trimmed.split(col_sep).collect();
        if parts.len() <= freq_col_idx.max(raw_col_idx) { continue; }

        let f_str = if dec_sep == ',' { parts[freq_col_idx].replace(',', ".") } else { parts[freq_col_idx].to_string() };
        let r_str = if dec_sep == ',' { parts[raw_col_idx].replace(',', ".") } else { parts[raw_col_idx].to_string() };

        if let (Ok(f), Ok(r)) = (f_str.trim().parse::<f64>(), r_str.trim().parse::<f64>()) {
            frequency.push(f);
            raw.push(r);
        }
    }

    if frequency.is_empty() {
        return Err(AutoEqError::CsvParse("No valid data found in CSV".into()));
    }

    Ok(ParsedCsv { frequency, raw })
}

/// Auto-detect column separator and decimal separator
fn find_csv_separators(csv: &str) -> Result<(char, char)> {
    let candidates = [',', ';', '\t', '|'];
    let numeric_lines: Vec<&str> = csv.lines()
        .filter(|l| l.starts_with(|c: char| c.is_ascii_digit()))
        .collect();

    if numeric_lines.is_empty() {
        return Err(AutoEqError::CsvParse("No numeric lines found".into()));
    }

    // Find which separators appear in ALL numeric lines
    let mut present: Vec<char> = candidates.iter().copied().filter(|&sep| {
        numeric_lines.iter().all(|line| line.contains(sep))
    }).collect();

    if present.is_empty() {
        // Single column? Try space separator
        if numeric_lines.iter().all(|line| line.contains(' ')) {
            return Ok((' ', '.'));
        }
        return Err(AutoEqError::CsvParse("Could not find column separator".into()));
    }

    if present.len() == 1 {
        let sep = present[0];
        if sep == ',' {
            return Ok((',', '.'));
        }
        return Ok((sep, '.'));
    }

    // Multiple candidates
    if present.contains(&',') {
        // Comma is likely the decimal separator
        present.retain(|&c| c != ',');
        if present.len() == 1 {
            return Ok((present[0], ','));
        }
        return Err(AutoEqError::CsvParse("Ambiguous separators".into()));
    }

    // No comma among candidates
    if present.len() == 1 {
        return Ok((present[0], '.'));
    }

    Err(AutoEqError::CsvParse("Ambiguous separators".into()))
}

/// Find column names from header line (if any)
fn find_csv_columns(csv: &str, col_sep: char) -> Option<Vec<String>> {
    let numeric_lines: Vec<&str> = csv.lines()
        .filter(|l| l.starts_with(|c: char| c.is_ascii_digit()) && l.contains(col_sep))
        .collect();

    if numeric_lines.is_empty() { return None; }

    // Determine column count from numeric lines
    let col_count = numeric_lines[0].split(col_sep).count();
    if !numeric_lines.iter().all(|l| l.split(col_sep).count() == col_count) {
        return None;
    }

    // Find a non-numeric line with the same column count
    for line in csv.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with(|c: char| c.is_ascii_digit()) || trimmed.starts_with('*') {
            continue;
        }
        if trimmed.split(col_sep).count() == col_count {
            return Some(trimmed.split(col_sep).map(|s| s.trim().to_string()).collect());
        }
    }

    None
}

/// Create CSV string from column data
pub fn create_csv(columns: &BTreeMap<String, Vec<f64>>) -> String {
    if columns.is_empty() { return String::new(); }

    let col_names: Vec<&String> = columns.keys().collect();
    let n_rows = col_names.iter().map(|k| columns[*k].len()).max().unwrap_or(0);

    let mut lines = Vec::with_capacity(n_rows + 1);

    // Header
    lines.push(col_names.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(","));

    // Data rows
    for i in 0..n_rows {
        let row: Vec<String> = col_names.iter().map(|k| {
            if i < columns[*k].len() {
                format!("{:.2}", columns[*k][i])
            } else {
                String::from("NaN")
            }
        }).collect();
        lines.push(row.join(","));
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_autoeq_format() {
        let csv = "frequency,raw,smoothed\n20.00,3.50,3.20\n50.00,2.10,2.00\n";
        let result = parse_csv(csv).unwrap();
        assert_eq!(result.frequency, vec![20.0, 50.0]);
        assert_eq!(result.raw, vec![3.5, 2.1]);
    }

    #[test]
    fn test_parse_rew_format() {
        let csv = "* REW V5.20\n* Freq(Hz) SPL(dB) Phase(degrees)\n20.0 3.5 10.0\n50.0 2.1 20.0\n";
        let result = parse_csv(csv).unwrap();
        assert_eq!(result.frequency, vec![20.0, 50.0]);
        assert_eq!(result.raw, vec![3.5, 2.1]);
    }

    #[test]
    fn test_parse_rew_2col_format() {
        let csv = "* REW V5.20\n* Freq(Hz) SPL(dB)\n20.0 3.5\n50.0 2.1\n";
        let result = parse_csv(csv).unwrap();
        assert_eq!(result.frequency, vec![20.0, 50.0]);
        assert_eq!(result.raw, vec![3.5, 2.1]);
    }

    #[test]
    fn test_parse_crinacle_format() {
        let csv = "Some header\nFrequency\tdB\tUnweighted\n20.0\t3.5\n50.0\t2.1\n";
        let result = parse_csv(csv).unwrap();
        assert_eq!(result.frequency, vec![20.0, 50.0]);
        assert_eq!(result.raw, vec![3.5, 2.1]);
    }

    #[test]
    fn test_parse_generic_csv() {
        let csv = "Freq,SPL\n20.0,3.5\n50.0,2.1\n";
        let result = parse_csv(csv).unwrap();
        assert_eq!(result.frequency, vec![20.0, 50.0]);
        assert_eq!(result.raw, vec![3.5, 2.1]);
    }

    #[test]
    fn test_parse_generic_semicolon() {
        let csv = "Freq;SPL\n20,0;3,5\n50,0;2,1\n";
        let result = parse_csv(csv).unwrap();
        assert_eq!(result.frequency, vec![20.0, 50.0]);
        assert_eq!(result.raw, vec![3.5, 2.1]);
    }

    #[test]
    fn test_parse_no_header() {
        let csv = "20.0,3.5\n50.0,2.1\n";
        let result = parse_csv(csv).unwrap();
        assert_eq!(result.frequency, vec![20.0, 50.0]);
        assert_eq!(result.raw, vec![3.5, 2.1]);
    }

    #[test]
    fn test_create_csv() {
        let mut columns = BTreeMap::new();
        columns.insert("frequency".to_string(), vec![20.0, 50.0]);
        columns.insert("raw".to_string(), vec![3.5, 2.1]);
        let csv = create_csv(&columns);
        assert!(csv.starts_with("frequency,raw"));
        assert!(csv.contains("20.00"));
    }

    #[test]
    fn test_find_csv_separators_comma() {
        let (col, dec) = find_csv_separators("20.0,3.5\n50.0,2.1\n").unwrap();
        assert_eq!(col, ',');
        assert_eq!(dec, '.');
    }

    #[test]
    fn test_find_csv_separators_semicolon() {
        let (col, dec) = find_csv_separators("20,0;3,5\n50,0;2,1\n").unwrap();
        assert_eq!(col, ';');
        assert_eq!(dec, ',');
    }
}
