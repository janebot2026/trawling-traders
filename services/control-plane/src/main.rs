use std::sync::Arc;
use tracing::{info, Level};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();

    info!("Starting Trawling Traders Control Plane...");

    // Database URL from env
    let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
        "postgres://postgres:postgres@localhost:5432/trawling_traders".to_string()
    });

    info!("Connecting to database...");

    // Initialize database
    let db = control_plane::db::init_db(&database_url).await?;
    info!("✓ Database connected");

    // Run app migrations (ignore_missing tolerates cedros-login's entries in _sqlx_migrations)
    info!("Running migrations...");
    let mut migrator = sqlx::migrate!("./migrations");
    migrator.ignore_missing = true;
    migrator.run(&db).await?;
    info!("✓ Migrations applied");

    // Initialize Cedros Login (embedded auth server)
    let mut login_error: Option<String> = None;
    let login_integration = match control_plane::cedros::login::full_router(db.clone()).await {
        Ok(integration) => {
            info!("✓ Cedros Login integration active");
            Some(integration)
        }
        Err(e) => {
            let msg = format!("{}", e);
            info!("⚠ Cedros Login using placeholder mode: {}", msg);
            login_error = Some(msg);
            None
        }
    };

    // Read max concurrent provisions from DB config (requires restart to change)
    let max_concurrent: usize = control_plane::config::get_config_or(
        &db,
        control_plane::config::keys::MAX_CONCURRENT_PROVISIONS,
        "3",
    )
    .await
    .parse()
    .unwrap_or(3);

    // Create app state with all components
    let mut app_state = control_plane::AppState::new(db.clone(), max_concurrent);
    if let Some(ref integration) = login_integration {
        app_state = app_state.with_jwt_service(integration.jwt_service.clone());
    }
    let state = Arc::new(app_state);
    info!(
        "✓ App state initialized (secrets: {}, metrics: {})",
        if state.secrets.is_encryption_active() {
            "encrypted"
        } else {
            "plaintext"
        },
        "active"
    );

    // Spawn orphan cleanup background task
    control_plane::provisioning::spawn_cleanup_task(db.clone(), state.secrets.clone());
    info!("✓ Orphan cleanup task spawned");

    // Spawn data retention cleanup task (events/metrics older than 30/90 days)
    control_plane::provisioning::spawn_data_retention_task(db.clone());
    info!("✓ Data retention cleanup task spawned");

    // Spawn offline bot checker (alerting)
    control_plane::alerting::spawn_offline_checker(db.clone(), state.alerts.clone());
    info!("✓ Offline bot checker spawned");

    // BUG-003: Spawn subscription cache cleanup (evicts stale entries every 5 min)
    state.spawn_subscription_cache_cleanup();
    info!("✓ Subscription cache cleanup task spawned");

    // Build router
    let app = build_router(state, db.clone(), login_integration, login_error).await?;

    // Start server
    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3000);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    info!("🚀 Control Plane listening on port {}", port);

    // REL-001: Graceful shutdown — let in-flight requests drain before exit.
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;

    Ok(())
}

/// Build the complete router
async fn build_router(
    state: Arc<control_plane::AppState>,
    pool: sqlx::PgPool,
    login_integration: Option<control_plane::cedros::login::LoginIntegration>,
    login_error: Option<String>,
) -> anyhow::Result<axum::Router> {
    use axum::http::{header, HeaderValue, Method};
    use axum::{
        routing::{get, patch, post},
        Router,
    };
    use tower_http::cors::CorsLayer;
    use tower_http::trace::TraceLayer;

    let allowed_origins = [
        "https://trawlingtraders.com",
        "https://www.trawlingtraders.com",
        "https://trawling-traders-web.vercel.app",
    ];

    let cors = CorsLayer::new()
        .allow_origin(
            allowed_origins
                .iter()
                .filter_map(|o| o.parse::<HeaderValue>().ok())
                .collect::<Vec<_>>(),
        )
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PATCH,
            Method::PUT,
            Method::DELETE,
            Method::OPTIONS,
        ])
        .allow_headers([
            header::CONTENT_TYPE,
            header::AUTHORIZATION,
            header::ACCEPT,
            header::COOKIE,
            // x-csrf-token removed: no CSRF validation is implemented (CP-009).
            // Re-add only when a CSRF middleware is in place.
        ])
        .allow_credentials(true);

    // App-facing routes (require auth + subscription + rate limit)
    let app_routes = Router::new()
        .route("/me", get(control_plane::handlers::bots::get_current_user))
        .route("/bots", get(control_plane::handlers::bots::list_bots))
        .route(
            "/bots/tradeable-assets",
            get(control_plane::handlers::bots::list_tradeable_assets),
        )
        .route(
            "/bots/assistant-options",
            get(control_plane::handlers::bots::list_ai_assistant_options),
        )
        .route(
            "/bots/name-availability",
            get(control_plane::handlers::bots::check_bot_name_availability),
        )
        .route(
            "/bots",
            post(control_plane::handlers::bots::create_bot).layer(
                axum::middleware::from_fn_with_state(
                    state.clone(),
                    control_plane::middleware::subscription::bot_create_limit_middleware,
                ),
            ),
        )
        .route("/bots/{id}", get(control_plane::handlers::bots::get_bot))
        .route(
            "/bots/{id}/config",
            patch(control_plane::handlers::bots::update_bot_config),
        )
        .route(
            "/bots/{id}/actions",
            post(control_plane::handlers::bots::bot_action),
        )
        .route(
            "/bots/{id}/metrics",
            get(control_plane::handlers::bots::get_metrics),
        )
        .route(
            "/bots/{id}/events",
            get(control_plane::handlers::bots::get_events),
        )
        .route(
            "/bots/{id}/chat/messages",
            get(control_plane::handlers::chat::get_bot_chat_messages),
        )
        .route(
            "/bots/{id}/chat/messages",
            post(control_plane::handlers::chat::post_bot_chat_message),
        )
        .route(
            "/account/settings",
            get(control_plane::handlers::settings::get_user_settings),
        )
        .route(
            "/account/display-name-availability",
            get(control_plane::handlers::settings::check_display_name_availability),
        )
        .route(
            "/account/settings",
            patch(control_plane::handlers::settings::update_user_settings),
        )
        .route(
            "/account/billing",
            get(control_plane::handlers::settings::get_billing_summary),
        )
        .route("/docs", get(control_plane::handlers::docs::get_docs))
        .route(
            "/docs/analytics",
            post(control_plane::handlers::docs::track_docs_event),
        )
        .route(
            "/reports/email-csv",
            post(control_plane::handlers::reports::request_email_csv_report),
        )
        .route(
            "/bots/{id}/openclaw-config",
            get(control_plane::handlers::openclaw_config::get_openclaw_config),
        )
        .route(
            "/bots/{id}/openclaw-config",
            post(control_plane::handlers::openclaw_config::update_openclaw_config),
        )
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            control_plane::middleware::subscription::subscription_middleware,
        ))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            control_plane::middleware::rate_limit::rate_limit_middleware,
        ))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            control_plane::middleware::auth_middleware,
        ))
        .with_state(state.clone());

    // Bot-facing routes (internal, from VPS)
    let bot_routes = Router::new()
        .route(
            "/bot/{id}/register",
            post(control_plane::handlers::sync::register_bot),
        )
        .route(
            "/bot/{id}/config",
            get(control_plane::handlers::sync::get_bot_config),
        )
        .route(
            "/bot/{id}/config_ack",
            post(control_plane::handlers::sync::ack_config),
        )
        .route(
            "/bot/{id}/wallet",
            post(control_plane::handlers::sync::report_wallet),
        )
        .route(
            "/bot/{id}/heartbeat",
            post(control_plane::handlers::sync::heartbeat),
        )
        .route(
            "/bot/{id}/events",
            post(control_plane::handlers::sync::ingest_events),
        )
        .route(
            "/bot/{id}/secrets",
            post(control_plane::handlers::sync::get_bot_secrets),
        )
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            control_plane::middleware::bot_auth_middleware,
        ))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            control_plane::middleware::rate_limit::bot_rate_limit_middleware,
        ))
        .with_state(state.clone());

    // Admin routes (require auth + admin check)
    let admin_routes = Router::new()
        .route("/config", get(control_plane::handlers::admin::list_config))
        .route(
            "/config/{key}",
            get(control_plane::handlers::admin::get_config),
        )
        .route(
            "/config",
            patch(control_plane::handlers::admin::update_config),
        )
        .route(
            "/config/audit",
            get(control_plane::handlers::admin::get_audit_log),
        )
        .route(
            "/config/test-webhook",
            post(control_plane::handlers::admin::test_webhook),
        )
        .route(
            "/config/sync-env",
            post(control_plane::handlers::admin::sync_env_to_db),
        )
        .route("/kpis", get(control_plane::handlers::admin::get_kpis))
        .route(
            "/provisioning/queue",
            get(control_plane::handlers::admin::get_provisioning_queue),
        )
        .route(
            "/audit",
            get(control_plane::handlers::admin::get_audit_log_entries),
        )
        // Admin bots list
        .route(
            "/bots",
            get(control_plane::handlers::admin_bots::list_admin_bots),
        )
        // Provisioning settings
        .route(
            "/provisioning/settings",
            get(control_plane::handlers::admin_provisioning::get_provisioning_settings),
        )
        .route(
            "/provisioning/settings",
            patch(control_plane::handlers::admin_provisioning::update_provisioning_settings),
        )
        // Presets CRUD
        .route(
            "/presets",
            get(control_plane::handlers::presets::list_presets),
        )
        .route(
            "/presets",
            post(control_plane::handlers::presets::create_preset),
        )
        .route(
            "/presets/{id}",
            patch(control_plane::handlers::presets::update_preset),
        )
        .route(
            "/presets/{id}",
            axum::routing::delete(control_plane::handlers::presets::delete_preset),
        )
        .layer(axum::middleware::from_fn(
            control_plane::middleware::admin_middleware,
        ))
        .layer(axum::middleware::from_fn_with_state(
            state.clone(),
            control_plane::middleware::auth_middleware,
        ))
        .with_state(state.clone());

    // Cedros Pay routes - try full integration, fallback to placeholder
    let mut pay_error: Option<String> = None;
    let cedros_routes = match control_plane::cedros::pay::full_router(pool).await {
        Ok(router) => {
            info!("✓ Cedros Pay full integration active");
            router
        }
        Err(e) => {
            let msg = format!("{}", e);
            info!("⚠ Cedros Pay using placeholder mode: {}", msg);
            pay_error = Some(msg);
            control_plane::cedros::pay::placeholder_routes()
        }
    };

    // Cedros Login routes - embedded auth server or placeholder
    let login_routes = match login_integration {
        Some(integration) => integration.router,
        None => control_plane::cedros::login::placeholder_routes(),
    };

    // Startup diagnostics — only show integration status, not raw error strings
    let diag = serde_json::json!({
        "login": if login_error.is_none() { "active" } else { "placeholder" },
        "login_error": login_error.as_ref().map(|_| "integration unavailable"),
        "pay": if pay_error.is_none() { "active" } else { "placeholder" },
        "pay_error": pay_error.as_ref().map(|_| "integration unavailable"),
    });
    let diagnostics_route = Router::new()
        .route(
            "/debug/startup",
            get(move || {
                let d = diag.clone();
                async move { axum::Json(d) }
            }),
        )
        .route(
            "/debug/echo-auth",
            get(|headers: axum::http::HeaderMap| async move {
                let auth = headers
                    .get(axum::http::header::AUTHORIZATION)
                    .and_then(|v| v.to_str().ok())
                    .map(|s| format!("[REDACTED] ({} chars)", s.len()));
                let origin = headers
                    .get(axum::http::header::ORIGIN)
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.to_string());
                axum::Json(serde_json::json!({
                    "authorization_present": auth.is_some(),
                    "authorization_preview": auth,
                    "origin": origin,
                    "cookie_present": headers.get(axum::http::header::COOKIE).is_some(),
                }))
            }),
        );
    let debug_routes_enabled = is_debug_routes_enabled(std::env::var("ENABLE_DEBUG_ROUTES").ok());

    // Health check routes (no auth)
    let health_routes = Router::new()
        .route("/healthz", get(control_plane::health::healthz))
        .route("/readyz", get(control_plane::health::readyz))
        .route("/health", get(control_plane::health::health_detail))
        .with_state(state.clone());

    // Build combined router
    let router = Router::new()
        .nest("/v1", app_routes)
        .nest("/v1", bot_routes)
        .nest("/v1/admin", admin_routes)
        .merge(cedros_routes.layer(axum::middleware::from_fn(
            |req: axum::http::Request<axum::body::Body>, next: axum::middleware::Next| async move {
                let method = req.method().clone();
                let uri = req.uri().clone();
                // Only log admin routes to avoid noise
                if uri.path().contains("/admin") {
                    let has_auth = req.headers().get(axum::http::header::AUTHORIZATION).is_some();
                    let has_api_key = req.headers().get("x-api-key").is_some();
                    tracing::info!(
                        %method, %uri, has_auth, has_api_key,
                        "cedros-pay request"
                    );
                    let resp = next.run(req).await;
                    tracing::info!(%method, %uri, status = %resp.status(), "cedros-pay response");
                    resp
                } else {
                    next.run(req).await
                }
            },
        ))) // cedros-pay applies its own route prefix
        .nest("/v1/auth", login_routes.layer(axum::middleware::from_fn(
            |req: axum::http::Request<axum::body::Body>, next: axum::middleware::Next| async move {
                let method = req.method().clone();
                let uri = req.uri().clone();
                let has_auth = req.headers().get(axum::http::header::AUTHORIZATION).is_some();
                tracing::info!(
                    %method, %uri, has_auth, auth_header = "[REDACTED]",
                    "cedros-login request"
                );
                let mut resp = next.run(req).await;
                tracing::info!(%method, %uri, status = %resp.status(), "cedros-login response");
                // Cache JWKS responses — keys only rotate on server restart
                if uri.path().ends_with("/.well-known/jwks.json") && resp.status().is_success() {
                    resp.headers_mut().insert(
                        axum::http::header::CACHE_CONTROL,
                        "public, max-age=300, stale-while-revalidate=60".parse().unwrap(),
                    );
                }
                resp
            },
        )))
        .nest("/v1", health_routes)
        .merge(if debug_routes_enabled {
            diagnostics_route
        } else {
            Router::new()
        })
        .layer(cors)
        .layer(TraceLayer::new_for_http());

    Ok(router)
}

/// REL-001: Wait for SIGINT (Ctrl-C) or SIGTERM (Docker stop) to initiate
/// graceful shutdown. In-flight requests will drain before the server exits.
async fn shutdown_signal() {
    let ctrl_c = tokio::signal::ctrl_c();

    #[cfg(unix)]
    {
        let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to register SIGTERM handler");
        tokio::select! {
            _ = ctrl_c => info!("Received SIGINT, shutting down..."),
            _ = sigterm.recv() => info!("Received SIGTERM, shutting down..."),
        }
    }

    #[cfg(not(unix))]
    {
        ctrl_c.await.ok();
        info!("Received SIGINT, shutting down...");
    }
}

fn is_debug_routes_enabled(value: Option<String>) -> bool {
    value
        .map(|v| v.trim().eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::is_debug_routes_enabled;

    #[test]
    fn debug_routes_disabled_by_default() {
        assert!(!is_debug_routes_enabled(None));
    }

    #[test]
    fn debug_routes_enabled_only_for_true() {
        assert!(is_debug_routes_enabled(Some("true".to_string())));
        assert!(is_debug_routes_enabled(Some(" TRUE ".to_string())));
        assert!(!is_debug_routes_enabled(Some("1".to_string())));
        assert!(!is_debug_routes_enabled(Some("false".to_string())));
    }
}
