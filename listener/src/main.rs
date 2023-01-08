#![deny(clippy::all, clippy::pedantic, clippy::nursery, clippy::cargo)]

use futures::stream::StreamExt;
use rand::Rng;
use sqlx::{query, MySqlPool};
use std::{env, sync::Arc};
use twilight_gateway::{
    cluster::{Cluster, ShardScheme},
    Event, Intents,
};
use twilight_model::id::Id;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    let token =
        env::var("DISCORD_TOKEN").expect("Failed to get DISCORD_TOKEN environment variable");
    let scheme = ShardScheme::Range {
        from: env::var("SHARDS_START")
            .expect("Failed to get SHARDS_START environment variable")
            .parse()
            .expect("Failed to parse SHARDS_START as u64"),
        to: env::var("SHARDS_END")
            .expect("Failed to get SHARDS_END environment variable")
            .parse()
            .expect("Failed to parse SHARDS_END as u64"),
        total: env::var("TOTAL_SHARDS")
            .expect("Failed to get TOTAL_SHARDS environment variable")
            .parse()
            .expect("Failed to parse TOTAL_SHARDS as u64"),
    };
    let redis_url = env::var("REDIS_URL").expect("Failed to get REDIS_URL environment variable");
    let mysql = env::var("DATABASE_URL").expect("Failed to get DATABASE_URL environment variable");
    println!("Connecting to database {mysql}");
    let db = sqlx::mysql::MySqlPoolOptions::new()
        .max_connections(50)
        .connect(&mysql)
        .await
        .expect("Failed to connect to database");

    let redis = redis::Client::open(redis_url).expect("Failed to connect to redis");

    let client = Arc::new(twilight_http::Client::new(token.clone()));

    let intents = Intents::GUILD_MESSAGES;

    let (cluster, mut events) = Cluster::builder(token.clone(), intents)
        .shard_scheme(scheme)
        .build()
        .await
        .expect("Failed to create discord cluster");

    let cluster = Arc::new(cluster);

    let cluster_spawn = cluster.clone();
    println!("Connecting to discord");
    tokio::spawn(async move {
        cluster_spawn.up().await;
    });

    let mut has_connected = false;
    while let Some((_shard_id, event)) = events.next().await {
        if !has_connected {
            has_connected = true;
            println!("Connected to discord");
        }
        let redis = redis.clone();
        let client = client.clone();
        let db = db.clone();
        tokio::spawn(async move {
            let redis = match redis.get_async_connection().await {
                Ok(val) => val,
                Err(e) => {
                    eprintln!("Redis connection error: {e}");
                    return;
                }
            };
            handle_event(event, db, redis, client).await;
        });
    }
}

async fn handle_event(
    event: Event,
    db: MySqlPool,
    mut redis: redis::aio::Connection,
    http: Arc<twilight_http::Client>,
) {
    if let Event::MessageCreate(msg) = event {
        if let Some(guild_id) = msg.guild_id {
            let has_sent: bool = redis::cmd("GET")
                .arg(format!("{guild_id}-{}", msg.author.id))
                .query_async(&mut redis)
                .await
                .unwrap_or(false);
            if !msg.author.bot && !has_sent {
                let xp_count = rand::thread_rng().gen_range(15..=25);
                if let Err(e) = query!(
                    "INSERT INTO levels (id, xp, guild) VALUES (?, ?, ?) ON DUPLICATE KEY UPDATE xp=xp+?",
                    msg.author.id.get(),
                    xp_count,
                    guild_id.get(),
                    xp_count
                )
                .execute(&db)
                .await
                {
                    eprintln!("SQL insert error: {e:?}");
                };
                if let Err(e) = redis::cmd("SET")
                    .arg(format!("{guild_id}-{}", msg.author.id))
                    .arg(true)
                    .arg("EX")
                    .arg(60)
                    .query_async::<redis::aio::Connection, ()>(&mut redis)
                    .await
                {
                    eprintln!("Redis error: {e}");
                    return;
                };
                let xp = match query!(
                    "SELECT xp FROM levels WHERE id = ? AND guild = ?",
                    msg.author.id.get(),
                    guild_id.get()
                )
                .fetch_one(&db)
                .await
                {
                    Ok(xp) => xp,
                    Err(e) => {
                        eprintln!("SQL select error: {e:?}");
                        return;
                    }
                }
                .xp;
                let level_info = mee6::LevelInfo::new(xp);
                let reward = match query!("SELECT id FROM role_rewards WHERE guild = ? AND requirement <= ? ORDER BY requirement DESC LIMIT 1", guild_id.get(), level_info.level())
                    .fetch_one(&db)
                    .await
                {
                    Ok(rw) => rw.id,
                    Err(e) => {
                        if matches!(e, sqlx::Error::RowNotFound) {return;}
                        eprintln!("SQL select error: {e:?}");
                        return;
                    }
                };
                http.add_guild_member_role(guild_id, msg.author.id, Id::new(reward))
                    .await
                    .ok();
            }
        }
    }
}
