use axum::{
    Json, Router,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use tower_http::cors::CorsLayer;

use crate::api::{self, EqualizeResult, ProcessParams};
use crate::peq::PeqResult;

// ── Request ──────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct EqualizeRequest {
    #[serde(default = "default_name")]
    pub name: String,
    pub frequency: Vec<f64>,
    pub raw: Vec<f64>,
    pub target_curve: Option<Vec<f64>>,
    #[serde(default = "default_peq_config")]
    pub peq_config: String,
    #[serde(default)]
    pub params: ProcessParams,
}

fn default_name() -> String {
    "headphone".to_string()
}

fn default_peq_config() -> String {
    "8_PEAKING_WITH_SHELVES".to_string()
}

// ── Response ─────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct EqualizeResponse {
    pub eq_result: EqualizeResult,
    pub parametric_eq: PeqResult,
    pub graphic_eq: String,
}

#[derive(Debug, Serialize)]
pub struct PeqResponse {
    pub parametric_eq: PeqResult,
}

#[derive(Debug, Serialize)]
pub struct GraphicEqResponse {
    pub graphic_eq: String,
}

#[derive(Debug, Serialize)]
pub struct ConfigsResponse {
    pub configs: Vec<&'static str>,
}

// ── Error ────────────────────────────────────────────────────────────

struct AppError(String);

impl From<String> for AppError {
    fn from(s: String) -> Self {
        AppError(s)
    }
}

impl From<tokio::task::JoinError> for AppError {
    fn from(e: tokio::task::JoinError) -> Self {
        AppError(format!("Task panicked: {}", e))
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        (StatusCode::INTERNAL_SERVER_ERROR, self.0).into_response()
    }
}

// ── Handlers ─────────────────────────────────────────────────────────

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({"status": "ok"}))
}

async fn get_configs() -> Json<ConfigsResponse> {
    let mut configs = api::get_available_configs();
    configs.sort();
    Json(ConfigsResponse { configs })
}

async fn equalize(Json(req): Json<EqualizeRequest>) -> Result<Json<EqualizeResponse>, AppError> {
    tokio::task::spawn_blocking(move || {
        let (eq_result, parametric_eq, graphic_eq) = api::equalize_data_full(
            &req.frequency,
            &req.raw,
            req.target_curve.as_deref(),
            &req.name,
            &req.params,
            &req.peq_config,
        )
        .map_err(|e| AppError(e.to_string()))?;

        Ok(Json(EqualizeResponse {
            eq_result,
            parametric_eq,
            graphic_eq,
        }))
    })
    .await?
}

async fn peq(Json(req): Json<EqualizeRequest>) -> Result<Json<PeqResponse>, AppError> {
    tokio::task::spawn_blocking(move || {
        let (_, parametric_eq, _) = api::equalize_data_full(
            &req.frequency,
            &req.raw,
            req.target_curve.as_deref(),
            &req.name,
            &req.params,
            &req.peq_config,
        )
        .map_err(|e| AppError(e.to_string()))?;

        Ok(Json(PeqResponse { parametric_eq }))
    })
    .await?
}

async fn graphic_eq(Json(req): Json<EqualizeRequest>) -> Result<Json<GraphicEqResponse>, AppError> {
    tokio::task::spawn_blocking(move || {
        let (_, _, graphic_eq) = api::equalize_data_full(
            &req.frequency,
            &req.raw,
            req.target_curve.as_deref(),
            &req.name,
            &req.params,
            &req.peq_config,
        )
        .map_err(|e| AppError(e.to_string()))?;

        Ok(Json(GraphicEqResponse { graphic_eq }))
    })
    .await?
}

// ── Router ───────────────────────────────────────────────────────────

pub fn create_router() -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/configs", get(get_configs))
        .route("/equalize", post(equalize))
        .route("/peq", post(peq))
        .route("/graphic-eq", post(graphic_eq))
        .layer(CorsLayer::permissive())
}