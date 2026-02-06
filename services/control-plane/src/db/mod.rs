use sqlx::{postgres::PgPoolOptions, Pool, Postgres};
use std::time::Duration;

pub type Db = Pool<Postgres>;

pub async fn init_db(database_url: &str) -> anyhow::Result<Db> {
    let pool = PgPoolOptions::new()
        .max_connections(20)
        .acquire_timeout(Duration::from_secs(3))
        .connect(database_url)
        .await?;

    Ok(pool)
}
