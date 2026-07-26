use std::sync::Arc;

use axum::{extract::State, Json};

use crate::config::UacConfig;
use crate::error::Result;
use crate::state::AppState;

use super::usb_update::update_usb_config;

pub async fn get_uac_config(State(state): State<Arc<AppState>>) -> Json<UacConfig> {
    Json(state.config.get().uac.clone())
}

pub async fn update_uac_config(
    State(state): State<Arc<AppState>>,
    Json(request): Json<UacConfig>,
) -> Result<Json<UacConfig>> {
    request.validate()?;
    let config = update_usb_config(&state, move |staged| {
        staged.uac = request;
        Ok(None)
    })
    .await?;
    Ok(Json(config.uac))
}
