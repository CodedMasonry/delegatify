#[allow(dead_code)]
use sqlx::migrate::MigrateError;

use crate::Error;

pub enum Permissions {
    Default,
    Basic,
}
// Row in table
#[derive(Debug, sqlx::FromRow)]
struct User {
    // #[sqlx(default)]
    // id: Option<i64>,
    #[sqlx(default)]
    permission: Option<i16>,
}

pub async fn migrate(pool: &sqlx::PgPool) -> Result<(), MigrateError> {
    sqlx::migrate!("./migrations").run(pool).await
}

pub async fn db_add_user(
    pool: &sqlx::PgPool,
    user_id: i64,
    level: Option<i16>,
) -> Result<(), Error> {
    let level = match level {
        Some(v) => v,
        None => 1,
    };
    let mut tx = pool.begin().await?;

    sqlx::query("INSERT INTO users (id, permission) VALUES ($1, $2)")
        .bind(user_id)
        .bind(level)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(())
}

pub async fn db_remove_user(pool: &sqlx::PgPool, user_id: i64) -> Result<(), Error> {
    let mut tx = pool.begin().await?;

    sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(user_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;
    Ok(())
}

// Fetches the User's permission; Returns none if user isn't in Database
pub async fn db_user_exists(pool: &sqlx::PgPool, user_id: i64) -> Result<bool, Error> {
    let result: Option<User> = sqlx::query_as("SELECT permission FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(pool)
        .await?;

    match result {
        Some(_) => Ok(true),
        None => Ok(false),
    }
}

// Fetches the User's permission; Returns none if user isn't in Database
pub async fn db_get_user_permission(
    pool: &sqlx::PgPool,
    user_id: i64,
) -> Result<Option<i16>, Error> {
    let result: Option<User> = sqlx::query_as("SELECT permission FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(pool)
        .await?;

    match result {
        Some(v) => Ok(Some(v.permission.unwrap())),
        None => Ok(None),
    }
}
