use sqlx::migrate::MigrateError;

pub async fn migrate(pool: &sqlx::PgPool) -> Result<(), MigrateError> {
    sqlx::migrate!("db/migrations").run(pool).await
}
