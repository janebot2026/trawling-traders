use axum::{
    extract::{Extension, Query, State},
    http::StatusCode,
    Json,
};
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    middleware::AuthContext,
    models::{
        AuthMethodsStatus, BillingSummaryResponse, NameAvailabilityResponse,
        UpdateUserSettingsRequest, UserSettingsResponse,
    },
    AppState,
};

#[derive(Debug, serde::Deserialize)]
pub struct DisplayNameQuery {
    pub display_name: Option<String>,
}

#[derive(Debug, sqlx::FromRow)]
struct UserSettingsRow {
    id: Uuid,
    email: Option<String>,
    name: Option<String>,
    default_persona: Option<crate::models::Persona>,
    picture: Option<String>,
    password_hash: Option<String>,
    google_id: Option<String>,
    apple_id: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, sqlx::FromRow)]
struct BillingRow {
    status: String,
    product_id: String,
    current_period_end: chrono::DateTime<chrono::Utc>,
}

fn normalize_display_name(value: Option<String>) -> Option<String> {
    value.and_then(|name| {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return None;
        }

        let bounded: String = trimmed.chars().take(80).collect();
        Some(bounded)
    })
}

fn to_response(row: UserSettingsRow) -> UserSettingsResponse {
    let default_persona = row
        .default_persona
        .unwrap_or_else(|| derive_default_persona(row.id));
    UserSettingsResponse {
        id: row.id,
        email: row.email,
        display_name: row.name,
        default_assistant_style: default_persona,
        picture: row.picture,
        auth_methods: AuthMethodsStatus {
            email_password: row.password_hash.is_some(),
            google: row.google_id.is_some(),
            apple: row.apple_id.is_some(),
        },
        created_at: row.created_at,
        updated_at: row.updated_at,
    }
}

fn derive_default_persona(user_id: Uuid) -> crate::models::Persona {
    super::helpers::derive_default_persona(user_id)
}

async fn ensure_default_persona(
    state: &AppState,
    user_id: Uuid,
) -> Result<crate::models::Persona, (StatusCode, String)> {
    let existing: Option<crate::models::Persona> =
        sqlx::query_scalar("SELECT default_persona FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_one(&state.db)
            .await
            .map_err(|e| {
                if matches!(e, sqlx::Error::RowNotFound) {
                    (StatusCode::NOT_FOUND, "User not found".to_string())
                } else {
                    (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
                }
            })?;

    if let Some(persona) = existing {
        return Ok(persona);
    }

    let derived = derive_default_persona(user_id);
    sqlx::query(
        "UPDATE users
         SET default_persona = $1, updated_at = NOW()
         WHERE id = $2 AND default_persona IS NULL",
    )
    .bind(derived)
    .bind(user_id)
    .execute(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(derived)
}

async fn load_user_settings_row(
    state: &AppState,
    user_id: Uuid,
) -> Result<UserSettingsRow, (StatusCode, String)> {
    sqlx::query_as::<_, UserSettingsRow>(
        "SELECT id, email, name, default_persona, picture, password_hash, google_id, apple_id, created_at, updated_at \
         FROM users \
         WHERE id = $1",
    )
    .bind(user_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        if matches!(e, sqlx::Error::RowNotFound) {
            (StatusCode::NOT_FOUND, "User not found".to_string())
        } else {
            (StatusCode::INTERNAL_SERVER_ERROR, e.to_string())
        }
    })
}

pub async fn get_user_settings(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<UserSettingsResponse>, (StatusCode, String)> {
    let user_id = Uuid::parse_str(&auth.user_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid user ID".to_string()))?;
    let _ = ensure_default_persona(&state, user_id).await?;
    let row = load_user_settings_row(&state, user_id).await?;
    Ok(Json(to_response(row)))
}

pub async fn update_user_settings(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Json(req): Json<UpdateUserSettingsRequest>,
) -> Result<Json<UserSettingsResponse>, (StatusCode, String)> {
    let user_id = Uuid::parse_str(&auth.user_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid user ID".to_string()))?;
    let display_name = normalize_display_name(req.display_name);
    let default_persona = req.default_assistant_style;

    sqlx::query(
        "UPDATE users
         SET name = $1, default_persona = COALESCE($2, default_persona), updated_at = NOW()
         WHERE id = $3",
    )
    .bind(display_name)
    .bind(default_persona)
    .bind(user_id)
    .execute(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let _ = ensure_default_persona(&state, user_id).await?;
    let row = load_user_settings_row(&state, user_id).await?;
    Ok(Json(to_response(row)))
}

pub async fn check_display_name_availability(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Query(query): Query<DisplayNameQuery>,
) -> Result<Json<NameAvailabilityResponse>, (StatusCode, String)> {
    let user_id = Uuid::parse_str(&auth.user_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid user ID".to_string()))?;
    let normalized_name = query
        .display_name
        .unwrap_or_default()
        .trim()
        .chars()
        .take(80)
        .collect::<String>();

    if normalized_name.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "Display name is required".to_string(),
        ));
    }

    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(
            SELECT 1 FROM users
            WHERE id != $1
              AND name IS NOT NULL
              AND LOWER(name) = LOWER($2)
        )",
    )
    .bind(user_id)
    .bind(&normalized_name)
    .fetch_one(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if !exists {
        return Ok(Json(NameAvailabilityResponse {
            available: true,
            normalized_name,
            suggested_name: None,
        }));
    }

    // Single query: fetch all taken suffixed names matching "name N" pattern
    let taken_names: Vec<String> = sqlx::query_scalar(
        "SELECT LOWER(name) FROM users
         WHERE id != $1
           AND name IS NOT NULL
           AND (LOWER(name) LIKE LOWER($2) || ' %')",
    )
    .bind(user_id)
    .bind(&normalized_name)
    .fetch_all(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let taken_set: std::collections::HashSet<String> = taken_names.into_iter().collect();
    let base_lower = normalized_name.to_lowercase();

    for idx in 2..=999 {
        let candidate = format!("{} {}", normalized_name, idx);
        if !taken_set.contains(&format!("{} {}", base_lower, idx)) {
            return Ok(Json(NameAvailabilityResponse {
                available: false,
                normalized_name,
                suggested_name: Some(candidate),
            }));
        }
    }

    Ok(Json(NameAvailabilityResponse {
        available: false,
        normalized_name,
        suggested_name: None,
    }))
}

fn plan_max_bots(product_id: &str) -> i32 {
    if product_id.contains("enterprise") {
        20
    } else if product_id.contains("pro") {
        4
    } else {
        1
    }
}

pub async fn get_billing_summary(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
) -> Result<Json<BillingSummaryResponse>, (StatusCode, String)> {
    let user_id = Uuid::parse_str(&auth.user_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid user ID".to_string()))?;

    let subscription = sqlx::query_as::<_, BillingRow>(
        "SELECT status, product_id, current_period_end
         FROM subscriptions
         WHERE user_id = $1
         ORDER BY current_period_end DESC
         LIMIT 1",
    )
    .bind(user_id.to_string())
    .fetch_optional(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let bot_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM bots WHERE user_id = $1 AND status != 'destroying'",
    )
    .bind(user_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let response = match subscription {
        Some(sub) => BillingSummaryResponse {
            status: sub.status,
            plan_code: sub.product_id.clone(),
            max_bots: plan_max_bots(&sub.product_id),
            bot_count: bot_count as i32,
            current_period_end: Some(sub.current_period_end),
        },
        None => BillingSummaryResponse {
            status: "inactive".to_string(),
            plan_code: "free".to_string(),
            max_bots: 1,
            bot_count: bot_count as i32,
            current_period_end: None,
        },
    };

    Ok(Json(response))
}

#[cfg(test)]
mod tests {
    use super::{normalize_display_name, plan_max_bots};

    #[test]
    fn normalize_display_name_trims_and_keeps_valid_input() {
        let normalized = normalize_display_name(Some("  Trader Lob  ".to_string()));
        assert_eq!(normalized.as_deref(), Some("Trader Lob"));
    }

    #[test]
    fn normalize_display_name_returns_none_for_blank_input() {
        let normalized = normalize_display_name(Some("   ".to_string()));
        assert_eq!(normalized, None);
    }

    #[test]
    fn plan_max_bots_for_pro_product() {
        assert_eq!(plan_max_bots("trader-pro-monthly"), 4);
    }

    #[test]
    fn plan_max_bots_defaults_to_free_when_unknown() {
        assert_eq!(plan_max_bots("starter"), 1);
    }
}
