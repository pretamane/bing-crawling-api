//! User Profiles module.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, FromRow};
use uuid::Uuid;
use utoipa::ToSchema;
use std::sync::Arc;
use crate::api::AppState;

#[derive(Debug, Serialize, Deserialize, Clone, ToSchema, FromRow)]
pub struct Profile {
    pub id: String,
    pub email: String,
    pub name: Option<String>,
    pub avatar_url: Option<String>,
    pub bio: Option<String>,
    pub created_at: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateProfileRequest {
    pub email: String,
    pub name: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateProfileRequest {
    pub name: Option<String>,
    pub avatar_url: Option<String>,
    pub bio: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ProfileResponse {
    pub success: bool,
    pub profile: Option<Profile>,
    pub message: Option<String>,
}

pub async fn init_profiles_table(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS profiles (
            id VARCHAR PRIMARY KEY,
            email VARCHAR NOT NULL UNIQUE,
            name VARCHAR,
            avatar_url TEXT,
            bio TEXT,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        );"#,
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn get_profile(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<ProfileResponse>, StatusCode> {
    let row: Option<Profile> = sqlx::query_as(
        r#"SELECT id, email, name, avatar_url, bio, 
           to_char(created_at, 'YYYY-MM-DD HH24:MI:SS') as created_at
           FROM profiles WHERE id = $1"#
    )
    .bind(&id)
    .fetch_optional(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    match row {
        Some(profile) => Ok(Json(ProfileResponse {
            success: true,
            profile: Some(profile),
            message: None,
        })),
        None => Err(StatusCode::NOT_FOUND),
    }
}

pub async fn create_profile(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateProfileRequest>,
) -> Result<Json<ProfileResponse>, StatusCode> {
    let id = Uuid::new_v4().to_string();
    
    sqlx::query("INSERT INTO profiles (id, email, name) VALUES ($1, $2, $3)")
        .bind(&id)
        .bind(&req.email)
        .bind(&req.name)
        .execute(&state.pool)
        .await
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    Ok(Json(ProfileResponse {
        success: true,
        profile: Some(Profile {
            id,
            email: req.email,
            name: req.name,
            avatar_url: None,
            bio: None,
            created_at: None,
        }),
        message: Some("Profile created".to_string()),
    }))
}

pub async fn update_profile(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<UpdateProfileRequest>,
) -> Result<Json<ProfileResponse>, StatusCode> {
    let result = sqlx::query(
        r#"UPDATE profiles SET 
           name = COALESCE($2, name),
           avatar_url = COALESCE($3, avatar_url),
           bio = COALESCE($4, bio)
           WHERE id = $1"#
    )
    .bind(&id)
    .bind(&req.name)
    .bind(&req.avatar_url)
    .bind(&req.bio)
    .execute(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    Ok(Json(ProfileResponse {
        success: true,
        profile: None,
        message: Some("Profile updated".to_string()),
    }))
}

pub async fn list_profiles(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<Profile>>, StatusCode> {
    let profiles: Vec<Profile> = sqlx::query_as(
        r#"SELECT id, email, name, avatar_url, bio,
           to_char(created_at, 'YYYY-MM-DD HH24:MI:SS') as created_at
           FROM profiles ORDER BY created_at DESC LIMIT 50"#
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(profiles))
}
