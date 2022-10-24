#![warn(clippy::all, clippy::pedantic, clippy::nursery)]

use dashmap::DashMap;
use futures::stream::StreamExt;
use rand::Rng;
use sqlx::{query, MySqlPool};
use std::{env, sync::Arc, time::Instant};
use twilight_gateway::{
    cluster::{Cluster, ShardScheme},
    Event, Intents,
};
use twilight_model::id::{
    marker::{RoleMarker, UserMarker},
    Id,
};

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
    let rewards: Vec<(u64, Id<RoleMarker>)> = query!("SELECT id, requirement FROM role_rewards",)
        .fetch_all(&db)
        .await
        .expect("Failed to get role rewards from database")
        .iter()
        .map(|v| {
            (
                v.requirement,
                Id::<RoleMarker>::new_checked(v.id)
                    .expect("One of your role rewards has an invalid role ID!")
                    .cast(),
            )
        })
        .collect::<Vec<(u64, Id<RoleMarker>)>>()
        .sort();
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
        tokio::spawn(handle_event(
            event,
            db.clone(),
            cooldown.clone(),
            rewards.clone(),
        ));
    }
}

async fn handle_event(
    event: Event,
    db: MySqlPool,
    cooldown: Arc<DashMap<Id<UserMarker>, Instant>>,
    rewards: Vec<(u64, Id<RoleMarker>)>,
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
            let xp = match query!("SELECT xp FROM levels WHERE id = ?", msg.author.id.get())
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
            for reward in rewards {}
        }
    }
}

async fn clean_cooldown(cooldown: Arc<DashMap<Id<UserMarker>, Instant>>) {
    loop {
        cooldown.retain(|_, time| time.elapsed() < std::time::Duration::from_secs(60));
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
}
