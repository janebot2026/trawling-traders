use axum::{
    extract::{Extension, Path, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

use crate::{middleware::AuthContext, models::*, AppState};

#[derive(Debug, Deserialize)]
struct OpenAiMessage {
    content: String,
}

#[derive(Debug, Deserialize)]
struct OpenAiChoice {
    message: OpenAiMessage,
}

#[derive(Debug, Deserialize)]
struct OpenAiResponse {
    choices: Vec<OpenAiChoice>,
}

#[derive(Debug, Deserialize)]
struct AnthropicTextBlock {
    text: String,
}

#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    content: Vec<AnthropicTextBlock>,
}

#[derive(Debug)]
struct LlmConfig {
    provider: String,
    model: String,
    api_key: String,
}

/// Parse `user_id` from auth context and delegate to the shared [`helpers::get_authorized_bot`].
async fn get_authorized_bot(
    db: &sqlx::PgPool,
    auth: &AuthContext,
    bot_id: Uuid,
) -> Result<Bot, (StatusCode, String)> {
    let user_id = Uuid::parse_str(&auth.user_id)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid user ID".to_string()))?;
    super::helpers::get_authorized_bot(db, bot_id, user_id).await
}

async fn load_llm_config(
    state: &AppState,
    bot_id: Uuid,
) -> Result<LlmConfig, (StatusCode, String)> {
    let openclaw_cfg = sqlx::query_as::<_, BotOpenClawConfig>(
        "SELECT * FROM bot_openclaw_config WHERE bot_id = $1",
    )
    .bind(bot_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if let Some(cfg) = openclaw_cfg {
        let api_key = if cfg.encrypted_llm_api_key.is_empty() {
            String::new()
        } else {
            state
                .secrets
                .decrypt(&cfg.encrypted_llm_api_key)
                .map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Failed to decrypt API key: {e}"),
                    )
                })?
        };

        if !api_key.is_empty() {
            let model = if cfg.llm_model.trim().is_empty() {
                default_model_for_provider(&cfg.llm_provider).to_string()
            } else {
                cfg.llm_model
            };
            return Ok(LlmConfig {
                provider: cfg.llm_provider,
                model,
                api_key,
            });
        }
    }

    let cfg = sqlx::query_as::<_, ConfigVersion>(
        "SELECT * FROM config_versions WHERE id = (SELECT desired_version_id FROM bots WHERE id = $1)",
    )
    .bind(bot_id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| match e {
        sqlx::Error::RowNotFound => (
            StatusCode::NOT_FOUND,
            "No bot config found. Configure LLM provider first.".to_string(),
        ),
        _ => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    })?;

    let api_key = if cfg.encrypted_llm_api_key.is_empty() {
        String::new()
    } else {
        state
            .secrets
            .decrypt(&cfg.encrypted_llm_api_key)
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to decrypt API key: {e}"),
                )
            })?
    };

    if api_key.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "No LLM API key configured for this bot.".to_string(),
        ));
    }

    let provider = cfg.llm_provider;
    let model = default_model_for_provider(&provider).to_string();

    Ok(LlmConfig {
        provider,
        model,
        api_key,
    })
}

fn default_model_for_provider(provider: &str) -> &'static str {
    match provider {
        "anthropic" => "claude-3-5-sonnet",
        _ => "gpt-4o-mini",
    }
}

async fn call_llm(
    http_client: &reqwest::Client,
    llm: &LlmConfig,
    system_prompt: &str,
    messages: &[BotChatMessage],
) -> Result<String, (StatusCode, String)> {
    let provider = llm.provider.to_lowercase();
    match provider.as_str() {
        "anthropic" => call_anthropic(http_client, llm, system_prompt, messages).await,
        "openai" | "openrouter" | "venice" => {
            call_openai_compatible(http_client, llm, system_prompt, messages).await
        }
        _ => Err((
            StatusCode::BAD_REQUEST,
            format!("Unsupported LLM provider: {}", llm.provider),
        )),
    }
}

async fn call_openai_compatible(
    http_client: &reqwest::Client,
    llm: &LlmConfig,
    system_prompt: &str,
    messages: &[BotChatMessage],
) -> Result<String, (StatusCode, String)> {
    let endpoint = match llm.provider.as_str() {
        "openrouter" => "https://openrouter.ai/api/v1/chat/completions",
        "venice" => "https://api.venice.ai/api/v1/chat/completions",
        _ => "https://api.openai.com/v1/chat/completions",
    };

    let mut payload_messages = Vec::with_capacity(messages.len() + 1);
    payload_messages.push(json!({
        "role": "system",
        "content": system_prompt,
    }));

    for msg in messages {
        payload_messages.push(json!({
            "role": msg.role,
            "content": msg.content,
        }));
    }

    let body = json!({
        "model": llm.model,
        "messages": payload_messages,
        "temperature": 0.3,
    });

    let response = http_client
        .post(endpoint)
        .bearer_auth(&llm.api_key)
        .json(&body)
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                format!("LLM request failed for {}: {}", llm.provider, e),
            )
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "unknown provider error".to_string());
        return Err((
            StatusCode::BAD_GATEWAY,
            format!("LLM provider error ({}): {}", status, body),
        ));
    }

    let parsed: OpenAiResponse = response.json().await.map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            format!("Invalid LLM response: {e}"),
        )
    })?;

    let content = parsed
        .choices
        .first()
        .map(|c| c.message.content.trim().to_string())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| {
            (
                StatusCode::BAD_GATEWAY,
                "LLM returned an empty response".to_string(),
            )
        })?;

    Ok(content)
}

async fn call_anthropic(
    http_client: &reqwest::Client,
    llm: &LlmConfig,
    system_prompt: &str,
    messages: &[BotChatMessage],
) -> Result<String, (StatusCode, String)> {
    let anthropic_messages: Vec<_> = messages
        .iter()
        .map(|m| {
            json!({
                "role": m.role,
                "content": m.content,
            })
        })
        .collect();

    let body = json!({
        "model": llm.model,
        "system": system_prompt,
        "max_tokens": 500,
        "messages": anthropic_messages,
    });

    let response = http_client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", &llm.api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&body)
        .send()
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                format!("LLM request failed for anthropic: {}", e),
            )
        })?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response
            .text()
            .await
            .unwrap_or_else(|_| "unknown provider error".to_string());
        return Err((
            StatusCode::BAD_GATEWAY,
            format!("Anthropic provider error ({}): {}", status, body),
        ));
    }

    let parsed: AnthropicResponse = response.json().await.map_err(|e| {
        (
            StatusCode::BAD_GATEWAY,
            format!("Invalid LLM response: {e}"),
        )
    })?;

    let content = parsed
        .content
        .iter()
        .map(|c| c.text.trim())
        .filter(|t| !t.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string();

    if content.is_empty() {
        return Err((
            StatusCode::BAD_GATEWAY,
            "LLM returned an empty response".to_string(),
        ));
    }

    Ok(content)
}

fn build_system_prompt(bot: &Bot) -> String {
    format!(
        "You are the assistant for trading bot '{name}' (persona: {persona:?}). \
         Give concise, practical answers about strategy, risk, trades, and configuration. \
         Never claim to have executed a trade unless the user explicitly asked and the platform confirmed it.",
        name = bot.name,
        persona = bot.persona,
    )
}

pub async fn get_bot_chat_messages(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(bot_id): Path<Uuid>,
) -> Result<Json<GetBotChatMessagesResponse>, (StatusCode, String)> {
    let _bot = get_authorized_bot(&state.db, &auth, bot_id).await?;

    let messages = sqlx::query_as::<_, BotChatMessage>(
        "SELECT * FROM bot_chat_messages WHERE bot_id = $1 ORDER BY created_at ASC LIMIT 200",
    )
    .bind(bot_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(GetBotChatMessagesResponse { messages }))
}

pub async fn post_bot_chat_message(
    State(state): State<Arc<AppState>>,
    Extension(auth): Extension<AuthContext>,
    Path(bot_id): Path<Uuid>,
    Json(req): Json<BotChatMessageCreateRequest>,
) -> Result<Json<PostBotChatMessageResponse>, (StatusCode, String)> {
    let bot = get_authorized_bot(&state.db, &auth, bot_id).await?;

    let content = req.content.trim();
    if content.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "Message content is required".to_string(),
        ));
    }
    if content.len() > 4000 {
        return Err((
            StatusCode::BAD_REQUEST,
            "Message content exceeds 4000 characters".to_string(),
        ));
    }

    let user_message = sqlx::query_as::<_, BotChatMessage>(
        "INSERT INTO bot_chat_messages (bot_id, role, content) VALUES ($1, 'user', $2) RETURNING *",
    )
    .bind(bot_id)
    .bind(content)
    .fetch_one(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let history = sqlx::query_as::<_, BotChatMessage>(
        "SELECT * FROM bot_chat_messages WHERE bot_id = $1 ORDER BY created_at DESC LIMIT 30",
    )
    .bind(bot_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let mut chronological = history;
    chronological.reverse();

    let llm = load_llm_config(&state, bot_id).await?;
    let system_prompt = build_system_prompt(&bot);
    let assistant_content = call_llm(&state.http_client, &llm, &system_prompt, &chronological).await?;

    let assistant_message = sqlx::query_as::<_, BotChatMessage>(
        "INSERT INTO bot_chat_messages (bot_id, role, content) VALUES ($1, 'assistant', $2) RETURNING *",
    )
    .bind(bot_id)
    .bind(assistant_content)
    .fetch_one(&state.db)
    .await
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(PostBotChatMessageResponse {
        user_message,
        assistant_message,
    }))
}
