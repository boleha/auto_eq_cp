#[derive(Debug, thiserror::Error)]
pub enum AutoEqError {
    #[error("CSV parse error: {0}")]
    CsvParse(String),
    #[error("Invalid input: {0}")]
    InvalidInput(String),
    #[error("Optimization finished (early stop)")]
    OptimizationFinished,
    #[error("Optimization failed: {0}")]
    OptimizationFailed(String),
    #[error("DSP error: {0}")]
    Dsp(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, AutoEqError>;
