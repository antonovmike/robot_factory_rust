use sqlx::Pool;
use sqlx::Postgres;
use sqlx::postgres::PgPool;
use std::sync::Arc;
use tokio::sync::OnceCell;

use crate::constants::DATABASE_URL;

static POOL: OnceCell<Arc<Pool<Postgres>>> = OnceCell::const_new();

pub async fn get_pool() -> Result<Arc<Pool<Postgres>>, sqlx::Error> {
    let pool = POOL.get_or_init(|| async {
        let pool = PgPool::connect(DATABASE_URL).await.expect("Failed to connect to database");
        Arc::new(pool)
    }).await;

    Ok(pool.clone())
}
