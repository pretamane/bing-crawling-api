//! Notifications module using Resend (FREE - 3K emails/month).

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
pub struct Notification {
    pub id: String,
    pub user_id: String,
    pub notification_type: String,
    pub subject: Option<String>,
    pub message: String,
    pub read: bool,
    pub created_at: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct SendNotificationRequest {
    pub user_id: String,
    pub to_email: String,
    pub subject: String,
    pub message: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct NotificationResponse {
    pub success: bool,
    pub notification_id: Option<String>,
    pub message: String,
}

pub async fn init_notifications_table(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS notifications (
            id VARCHAR PRIMARY KEY,
            user_id VARCHAR NOT NULL,
            notification_type VARCHAR(20) DEFAULT 'email',
            subject VARCHAR(255),
            message TEXT NOT NULL,
            read BOOLEAN DEFAULT FALSE,
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        );"#,
    )
    .execute(pool)
    .await?;
    Ok(())
}

async fn send_email_via_resend(to: &str, subject: &str, body: &str) -> Result<String, String> {
    let api_key = std::env::var("RESEND_API_KEY")
        .map_err(|_| "RESEND_API_KEY not set - email simulated")?;

    let client = reqwest::Client::new();
    let payload = serde_json::json!({
        "from": "Crawler <notifications@resend.dev>",
        "to": [to],
        "subject": subject,
        "text": body
    });

    let response = client
        .post("https://api.resend.com/emails")
        .header("Authorization", format!("Bearer {}", api_key))
        .json(&payload)
        .send()
        .await
        .map_err(|e| format!("Resend error: {}", e))?;

    if response.status().is_success() {
        Ok("Email sent".to_string())
    } else {
        Err("Resend failed".to_string())
    }
}

use crate::auth::AuthUser;

pub async fn send_notification(
    State(state): State<Arc<AppState>>,
    _user: AuthUser, // Require auth, but currently anyone can send to anyone (or we could enforce admin role)
    Json(req): Json<SendNotificationRequest>,
) -> Result<Json<NotificationResponse>, StatusCode> {
    let notification_id = Uuid::new_v4().to_string();
    
    let message = match send_email_via_resend(&req.to_email, &req.subject, &req.message).await {
        Ok(msg) => msg,
        Err(e) => format!("Stored (email skipped: {})", e),
    };

    sqlx::query(
        "INSERT INTO notifications (id, user_id, notification_type, subject, message) VALUES ($1, $2, 'email', $3, $4)"
    )
    .bind(&notification_id)
    .bind(&req.user_id)
    .bind(&req.subject)
    .bind(&req.message)
    .execute(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(NotificationResponse {
        success: true,
        notification_id: Some(notification_id),
        message,
    }))
}

pub async fn get_notifications(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
) -> Result<Json<Vec<Notification>>, StatusCode> {
    
    // Workaround: Acquire connection and clean it
    let mut conn = state.pool.acquire().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    use sqlx::Executor; // trait import
    conn.execute("DEALLOCATE ALL").await.ok();

    let notifications: Vec<Notification> = sqlx::query_as(
        r#"SELECT id, user_id, notification_type, subject, message, read,
           to_char(created_at, 'YYYY-MM-DD HH24:MI:SS') as created_at
           FROM notifications WHERE user_id = $1 ORDER BY created_at DESC LIMIT 50"#
    )
    .bind(&user.id)
    .fetch_all(&mut *conn)
    .await
    .map_err(|e| {
        println!("🔥 DB Error: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    Ok(Json(notifications))
}

pub async fn mark_as_read(
    State(state): State<Arc<AppState>>,
    user: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<NotificationResponse>, StatusCode> {
    // Ensure the notification belongs to the user
    let result = sqlx::query("UPDATE notifications SET read = TRUE WHERE id = $1 AND user_id = $2")
        .bind(&id)
        .bind(&user.id)
        .execute(&state.pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if result.rows_affected() == 0 {
        return Err(StatusCode::NOT_FOUND);
    }

    Ok(Json(NotificationResponse {
        success: true,
        notification_id: Some(id),
        message: "Marked as read".to_string(),
    }))
}
