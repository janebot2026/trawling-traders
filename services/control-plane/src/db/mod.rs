use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use std::time::Duration;

pub type Db = Pool<Postgres>;

pub async fn init_db(database_url: &str) -> anyhow::Result<Db> {
    let pool = PgPoolOptions::new()
        .max_connections(20)
        .acquire_timeout(Duration::from_secs(3))
        .connect(database_url)
        .await?;

    // Run migrations
    sqlx::migrate!("./migrations").run(&pool).await?;

    Ok(pool)
}

// Note: Migrations are managed via sqlx::migrate!("./migrations")
// See migrations/001_initial_schema.sql for the actual schema
