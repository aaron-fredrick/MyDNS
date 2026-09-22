use anyhow::Context;
use sqlx::SqlitePool;

/// Looks up a user's hashed password by username.
pub async fn find_user_hash(pool: &SqlitePool, username: &str) -> anyhow::Result<Option<String>> {
    sqlx::query_scalar::<_, String>("SELECT password_hash FROM users WHERE username = ?")
        .bind(username)
        .fetch_optional(pool)
        .await
        .context("Failed to query user")
}

/// Inserts the admin user if not already present.
pub async fn seed_admin(
    pool: &SqlitePool,
    username: &str,
    password_hash: &str,
) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO users (username, password_hash) VALUES (?, ?) \
         ON CONFLICT(username) DO NOTHING",
    )
    .bind(username)
    .bind(password_hash)
    .execute(pool)
    .await
    .context("Failed to seed admin user")?;
    Ok(())
}
