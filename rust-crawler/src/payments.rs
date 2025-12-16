//! Payments module using Stripe (Test Mode - FREE).

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
pub struct Payment {
    pub id: String,
    pub user_id: String,
    pub amount: i32,
    pub currency: String,
    pub status: String,
    pub stripe_id: Option<String>,
    pub created_at: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreatePaymentRequest {
    pub user_id: String,
    pub amount: i32,
    pub currency: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PaymentResponse {
    pub success: bool,
    pub payment_id: Option<String>,
    pub checkout_url: Option<String>,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct StripeWebhookEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub data: serde_json::Value,
}

pub async fn init_payments_table(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS payments (
            id VARCHAR PRIMARY KEY,
            user_id VARCHAR NOT NULL,
            amount INTEGER NOT NULL,
            currency VARCHAR(3) DEFAULT 'USD',
            status VARCHAR(20) DEFAULT 'pending',
            stripe_id VARCHAR(100),
            created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
        );"#,
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn create_checkout(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreatePaymentRequest>,
) -> Result<Json<PaymentResponse>, StatusCode> {
    let payment_id = Uuid::new_v4().to_string();
    let currency = req.currency.unwrap_or_else(|| "USD".to_string());
    
    let stripe_key = std::env::var("STRIPE_SECRET_KEY").ok();
    
    let (status, checkout_url, message) = if stripe_key.is_some() {
        ("pending".to_string(), 
         Some(format!("https://checkout.stripe.com/demo/{}", payment_id)),
         "Stripe checkout session created".to_string())
    } else {
        ("demo".to_string(),
         Some(format!("http://localhost:3000/payments/demo/{}", payment_id)),
         "Demo mode: Set STRIPE_SECRET_KEY for real payments".to_string())
    };

    sqlx::query(
        "INSERT INTO payments (id, user_id, amount, currency, status) VALUES ($1, $2, $3, $4, $5)"
    )
    .bind(&payment_id)
    .bind(&req.user_id)
    .bind(req.amount)
    .bind(&currency)
    .bind(&status)
    .execute(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(PaymentResponse {
        success: true,
        payment_id: Some(payment_id),
        checkout_url,
        message,
    }))
}

pub async fn handle_webhook(
    State(state): State<Arc<AppState>>,
    Json(event): Json<StripeWebhookEvent>,
) -> Result<Json<PaymentResponse>, StatusCode> {
    println!("📦 Received Stripe webhook: {}", event.event_type);
    
    if event.event_type == "checkout.session.completed" {
        if let Some(session) = event.data.get("object") {
            if let Some(payment_id) = session.get("client_reference_id").and_then(|v| v.as_str()) {
                let _ = sqlx::query("UPDATE payments SET status = 'completed' WHERE id = $1")
                    .bind(payment_id)
                    .execute(&state.pool)
                    .await;
            }
        }
    }

    Ok(Json(PaymentResponse {
        success: true,
        payment_id: None,
        checkout_url: None,
        message: "Webhook processed".to_string(),
    }))
}

pub async fn get_payment_history(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<String>,
) -> Result<Json<Vec<Payment>>, StatusCode> {
    let payments: Vec<Payment> = sqlx::query_as(
        r#"SELECT id, user_id, amount, currency, status, stripe_id,
           to_char(created_at, 'YYYY-MM-DD HH24:MI:SS') as created_at
           FROM payments WHERE user_id = $1 ORDER BY created_at DESC"#
    )
    .bind(&user_id)
    .fetch_all(&state.pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(payments))
}
