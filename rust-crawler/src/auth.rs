//! Authentication module using Supabase JWT verification.

use axum::{
    http::StatusCode,
    Json,
};
use jsonwebtoken::{decode, DecodingKey, Validation, Algorithm};
use serde::{Deserialize, Serialize};

/// JWT Claims from Supabase
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,
    pub email: Option<String>,
    pub role: Option<String>,
    pub exp: usize,
    pub iat: usize,
}

/// User context extracted from JWT
#[derive(Debug, Clone, Serialize)]
pub struct AuthUser {
    pub id: String,
    pub email: Option<String>,
    pub role: String,
}

/// Auth Response
#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub message: String,
    pub user: Option<AuthUser>,
}

/// Verify JWT token and extract claims
pub fn verify_token(token: &str, secret: &str) -> Result<Claims, String> {
    let key = DecodingKey::from_secret(secret.as_bytes());
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true;
    
    decode::<Claims>(token, &key, &validation)
        .map(|data| data.claims)
        .map_err(|e| format!("JWT verification failed: {}", e))
}

/// Extract Bearer token from Authorization header
pub fn extract_bearer_token(auth_header: &str) -> Option<&str> {
    if auth_header.starts_with("Bearer ") {
        Some(&auth_header[7..])
    } else {
        None
    }
}

/// Health check for auth service
pub async fn auth_status() -> Json<AuthResponse> {
    Json(AuthResponse {
        message: "Auth service ready. Use Supabase client for login/register.".to_string(),
        user: None,
    })
}


use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{request::Parts, header},
};

#[async_trait]
impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, Json<AuthResponse>);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .ok_or_else(|| {
                (
                    StatusCode::UNAUTHORIZED,
                    Json(AuthResponse {
                        message: "Missing Authorization header".to_string(),
                        user: None,
                    }),
                )
            })?;

        let token = extract_bearer_token(auth_header).ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(AuthResponse {
                    message: "Invalid Authorization header format".to_string(),
                    user: None,
                }),
            )
        })?;

        let secret = std::env::var("SUPABASE_JWT_SECRET")
            .unwrap_or_else(|_| "demo-secret".to_string());

        let claims = verify_token(token, &secret).map_err(|e| {
            println!("⚠️ Auth Failed: {}", e);
            (
                StatusCode::UNAUTHORIZED,
                Json(AuthResponse {
                    message: "Invalid or expired token".to_string(),
                    user: None,
                }),
            )
        })?;

        Ok(AuthUser {
            id: claims.sub,
            email: claims.email,
            role: claims.role.unwrap_or_else(|| "user".to_string()),
        })
    }
}

