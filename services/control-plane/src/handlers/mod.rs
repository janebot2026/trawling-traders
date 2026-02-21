// TODO(CP-012): Handler return types are inconsistent across modules. Some return
// `Result<Json<T>, (StatusCode, String)>`, others return `impl IntoResponse` or plain
// `Json<T>`. Target pattern is `Result<Json<T>, (StatusCode, String)>` uniformly.
// Refactor incrementally per handler file rather than in one pass.

pub mod admin;
pub mod admin_bots;
pub mod admin_provisioning;
pub mod bots;
pub mod chat;
pub mod docs;
pub mod helpers;
pub mod openclaw_config;
pub mod presets;
pub mod reports;
pub mod settings;
pub mod sync;
