use std::env;
use redis::{self, RedisResult, aio::Connection};

pub async fn connect() -> Connection {
    let db_url = env::var("REDIS_URI")
        .expect("Redis connection string missing");

    let conn = redis::Client::open(db_url)
        .expect("Unable to open connection to Redis");

    let async_conn = conn.get_async_connection()
        .await
        .expect("Unable to open async connection to Redis");

    return async_conn;
}

pub async fn add_user_to_channels(user_id: &str, channels: Vec<String>) -> RedisResult<()> {
    let mut conn = connect().await;
    for channel in channels {
        let _: () = redis::cmd("SADD")
            .arg(channel)
            .arg(user_id)
            .query_async(&mut conn)
            .await?;
    }

    return Ok(());
}

pub async fn remove_user(user_id: &String, channels: Vec<String>) -> RedisResult<()> {
    let mut conn = connect().await;
    for channel in channels {
        let _: () = redis::cmd("SREM")
            .arg(channel)
            .arg(user_id)
            .query_async(&mut conn)
            .await?;
    }

    return Ok(());
}

pub async fn get_channel_users(id: &str) -> RedisResult<Vec<String>>{
    let mut conn = connect().await;
    let members = redis::cmd("SMEMBERS")
        .arg(id)
        .query_async(&mut conn)
        .await?;

    return Ok(members);
}
