use crate::App;
use axum::{
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use std::sync::Arc;

pub struct Auth {
    pub client_id: u32,
    pub client_secret: String,
}

pub fn validate_authentication_data(
    state: Arc<App>,
    headers: &HeaderMap,
) -> Result<Auth, impl IntoResponse> {
    let client_id = headers.get("client_id");
    let client_secret = headers.get("client_secret");

    if client_id.is_none() || client_secret.is_none() {
        return Err((StatusCode::UNAUTHORIZED, "Missing authentication data"));
    }

    let client_id = match client_id.unwrap().to_str() {
        Ok(id) => match id.parse::<u32>() {
            Ok(number) => number,
            Err(_) => return Err((StatusCode::UNAUTHORIZED, "Your client id is not a number")),
        },
        Err(_) => return Err((StatusCode::UNAUTHORIZED, "Invalid client_id formatting")),
    };

    let client_secret = match client_secret.unwrap().to_str() {
        Ok(secret) => secret,
        Err(_) => return Err((StatusCode::UNAUTHORIZED, "Invalid client_secret formatting")),
    };

    let matching_secret = state.authentication.get(&client_id);
    if matching_secret.is_none() || matching_secret.unwrap() != client_secret {
        return Err((StatusCode::UNAUTHORIZED, "Invalid client_secret formatting"));
    }

    return Ok(Auth {
        client_id,
        client_secret: client_secret.to_string(),
    });
}
