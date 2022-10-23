#![warn(clippy::all, clippy::pedantic, clippy::nursery)]

use dashmap::DashMap;
use futures::stream::StreamExt;
use rand::Rng;
use sqlx::{MySqlPool, query};
use std::{env, sync::Arc, time::Instant};
use twilight_gateway::{
    cluster::{Cluster, ShardScheme},
    Event, Intents,
};
use twilight_model::id::{marker::UserMarker, Id};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    let token =
        env::var("DISCORD_TOKEN").expect("Failed to get DATABASE_TOKEN environment variable");
    let mysql = env::var("DATABASE_URL").expect("Failed to get DATABASE_URL environment variable");
    let db = sqlx::mysql::MySqlPoolOptions::new()
        .max_connections(50)
        .connect(&mysql)
        .await
        .expect("Failed to connect to database");
    let cooldown: Arc<DashMap<Id<UserMarker>, Instant>> = Arc::new(DashMap::new());
    let scheme = ShardScheme::Range {
        from: 0,
        to: 0,
        total: 1,
    };

    let intents = Intents::GUILD_MESSAGES;

    let (cluster, mut events) = Cluster::builder(token.clone(), intents)
        .shard_scheme(scheme)
        .build()
        .await
        .expect("Failed to create discord cluster");

    let cluster = Arc::new(cluster);

    let cluster_spawn = cluster.clone();

    tokio::spawn(async move {
        cluster_spawn.up().await;
    });

    tokio::spawn(clean_cooldown(cooldown.clone()));

    while let Some((_shard_id, event)) = events.next().await {
        tokio::spawn(handle_event(event, db.clone(), cooldown.clone()));
    }
}

async fn handle_event(
    event: Event,
    db: MySqlPool,
    cooldown: Arc<DashMap<Id<UserMarker>, Instant>>,
) {
    if let Event::MessageCreate(msg) = event {
        if !msg.author.bot && cooldown.get(&msg.author.id).is_none() {
            let xp_count = rand::thread_rng().gen_range(15..=25);
            if let Err(e) = query!(
                "INSERT INTO levels (id, xp) VALUES (?, ?) ON DUPLICATE KEY UPDATE xp=xp+?",
                msg.author.id.get(),
                xp_count,
                xp_count 
            )
            .execute(&db)
            .await
            {
                eprintln!("SQL insert error: {e:?}");
            };
            cooldown.insert(msg.author.id, Instant::now());

            let role_rewards = query!("SELECT id FROM role_rewards WHERE requirement <= ? ORDER BY requirement LIMIT 1", level_info.level()).fetch_one(&state.db).await;
        }
    }
}

async fn clean_cooldown(cooldown: Arc<DashMap<Id<UserMarker>, Instant>>) {
    loop {
        cooldown.retain(|_, time| time.elapsed() < std::time::Duration::from_secs(60));
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
}
