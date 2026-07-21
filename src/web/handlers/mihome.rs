use axum::{
    extract::{Query, State},
    Json,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tracing::error;

use crate::error::{AppError, Result};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct MiHomeApiQuery {
    pub r#type: String,
    pub did: Option<String>,
    pub action: Option<String>,
    pub value: Option<String>,
    pub timestamp: String,
    pub sign: String,
}

fn generate_sign(did: &str, action: &str, value: &str, timestamp: &str, salt: &str) -> String {
    let sign_string = format!("{}{}{}{}{}", did, action, value, timestamp, salt);
    let mut hasher = Sha256::new();
    hasher.update(sign_string.as_bytes());
    hasher.finalize().iter().map(|b| format!("{:02x}", b)).collect::<String>()
}

fn verify_sign(query: &MiHomeApiQuery, salt: &str) -> bool {
    let did = query.did.as_deref().unwrap_or("");
    let action = query.action.as_deref().unwrap_or("");
    let value = query.value.as_deref().unwrap_or("");
    
    let expected_sign = generate_sign(did, action, value, &query.timestamp, salt);
    expected_sign == query.sign
}

pub async fn get_mihome_config(State(state): State<Arc<AppState>>) -> Json<crate::config::MiHomeConfig> {
    let config = state.config.get();
    Json(config.mihome.clone())
}

pub async fn update_mihome_config(
    State(state): State<Arc<AppState>>,
    Json(update): Json<crate::config::MiHomeConfig>,
) -> Result<Json<crate::config::MiHomeConfig>> {
    state.config.update(|config| {
        config.mihome = update.clone();
    }).await?;
    
    Ok(Json(update))
}

#[derive(Debug, Deserialize)]
pub struct MiHomeCheckQuery {
    pub api_url: String,
}

pub async fn check_mihome_server(
    Query(query): Query<MiHomeCheckQuery>,
) -> Result<Json<serde_json::Value>> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()
        .map_err(|e| {
            error!("创建 HTTP 客户端失败: {}", e);
            AppError::Internal(format!("创建 HTTP 客户端失败: {}", e))
        })?;
    
    let response = client
        .get(&query.api_url)
        .send()
        .await
        .map_err(|e| {
            error!("米家服务连接失败: {}", e);
            AppError::BadRequest(format!("米家服务连接失败: {}", e))
        })?;
    
    if !response.status().is_success() {
        return Err(AppError::BadRequest(format!("米家服务返回错误状态码: {}", response.status())));
    }
    
    let body = response.text().await.map_err(|e| {
        error!("读取米家服务响应失败: {}", e);
        AppError::Internal(format!("读取米家服务响应失败: {}", e))
    })?;
    
    let valid = body.contains("mihome server");
    
    Ok(Json(serde_json::json!({
        "valid": valid,
        "message": if valid { "米家服务正常" } else { "无法识别的米家服务响应" }
    })))
}

pub async fn mihome_api_proxy(
    State(state): State<Arc<AppState>>,
    Query(query): Query<MiHomeApiQuery>,
) -> Result<Json<serde_json::Value>> {
    let config = state.config.get();
    
    if !config.mihome.enabled {
        return Err(AppError::BadRequest("米家功能未启用".into()));
    }
    
    if config.mihome.api_key.is_empty() {
        return Err(AppError::BadRequest("米家 API 密钥未配置".into()));
    }
    
    if !verify_sign(&query, &config.mihome.api_key) {
        return Err(AppError::BadRequest("签名验证失败".into()));
    }
    
    let client = reqwest::Client::new();
    let base_url = &config.mihome.api_url;
    
    let mut request_url = format!("{}/api?type={}", base_url, query.r#type);
    
    if let Some(ref did) = query.did {
        request_url.push_str(&format!("&did={}", did));
    }
    if let Some(ref action) = query.action {
        request_url.push_str(&format!("&action={}", action));
    }
    if let Some(ref value) = query.value {
        request_url.push_str(&format!("&value={}", value));
    }
    request_url.push_str(&format!("&timestamp={}&sign={}", query.timestamp, query.sign));
    
    let response = client
        .get(&request_url)
        .send()
        .await
        .map_err(|e| {
            error!("米家 API 请求失败: {}", e);
            AppError::Internal(format!("米家 API 请求失败: {}", e))
        })?;
    
    let status = response.status();
    let body = response.text().await.map_err(|e| {
        error!("读取米家 API 响应失败: {}", e);
        AppError::Internal(format!("读取米家 API 响应失败: {}", e))
    })?;
    
    if !status.is_success() {
        return Err(AppError::Internal(format!("米家 API 返回错误: {} - {}", status, body)));
    }
    
    let json: serde_json::Value = serde_json::from_str(&body).map_err(|e| {
        error!("解析米家 API 响应失败: {}", e);
        AppError::Internal(format!("解析米家 API 响应失败: {}", e))
    })?;
    
    Ok(Json(json))
}

pub async fn mihome_control_proxy(
    State(state): State<Arc<AppState>>,
    Query(query): Query<MiHomeApiQuery>,
) -> Result<Json<serde_json::Value>> {
    let config = state.config.get();
    
    if !config.mihome.enabled {
        return Err(AppError::BadRequest("米家功能未启用".into()));
    }
    
    if config.mihome.api_key.is_empty() {
        return Err(AppError::BadRequest("米家 API 密钥未配置".into()));
    }
    
    if !verify_sign(&query, &config.mihome.api_key) {
        return Err(AppError::BadRequest("签名验证失败".into()));
    }
    
    let client = reqwest::Client::new();
    let base_url = &config.mihome.api_url;
    
    let mut request_url = format!("{}/api?type={}", base_url, query.r#type);
    
    if let Some(ref did) = query.did {
        request_url.push_str(&format!("&did={}", did));
    }
    if let Some(ref action) = query.action {
        request_url.push_str(&format!("&action={}", action));
    }
    if let Some(ref value) = query.value {
        request_url.push_str(&format!("&value={}", value));
    }
    request_url.push_str(&format!("&timestamp={}&sign={}", query.timestamp, query.sign));
    
    let response = client
        .post(&request_url)
        .send()
        .await
        .map_err(|e| {
            error!("米家控制 API 请求失败: {}", e);
            AppError::Internal(format!("米家控制 API 请求失败: {}", e))
        })?;
    
    let status = response.status();
    let body = response.text().await.map_err(|e| {
        error!("读取米家控制 API 响应失败: {}", e);
        AppError::Internal(format!("读取米家控制 API 响应失败: {}", e))
    })?;
    
    if !status.is_success() {
        return Err(AppError::Internal(format!("米家控制 API 返回错误: {} - {}", status, body)));
    }
    
    let json: serde_json::Value = serde_json::from_str(&body).map_err(|e| {
        error!("解析米家控制 API 响应失败: {}", e);
        AppError::Internal(format!("解析米家控制 API 响应失败: {}", e))
    })?;
    
    Ok(Json(json))
}