use std::env;
use sqlx::postgres::PgPoolOptions;
use anyhow::Result;

#[derive(sqlx::Type)]
enum ChannelUserStatus {
    ACTIVE,
    INACTIVE
}

impl ChannelUserStatus {
    fn is_active(&self) -> bool {
        match *self {
            ChannelUserStatus::ACTIVE => true,
            _ => false
        }
    }
}

#[derive(sqlx::FromRow)]
struct UserChannel {
    pub user_id: String,
    pub channel_id: String,
    id: i32,
    status: ChannelUserStatus
}

pub async fn fetch_channels(user_id: &String) -> Result<Vec<String>> {
    let conn_str = env::var("DATABASE_URL")
        .expect("Postgres connection string missing");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&conn_str)
        .await?;

    let user_channels = sqlx::query_as::<_, UserChannel>("SELECT * FROM \"UserChannel\" where user_id = $1")
        .bind(&user_id)
        .fetch_all(&pool)
        .await?;

    let channels: Vec<String> = user_channels
        .iter()
        .filter(|channel| channel.status.is_active())
        .map(|channel| channel.channel_id.clone())
        .collect();

    return Ok(channels);
}
