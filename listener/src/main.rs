#![warn(clippy::all, clippy::pedantic, clippy::nursery)]

use futures::stream::StreamExt;
use rand::Rng;
use sqlx::MySqlPool;
use std::{env, sync::Arc};
use twilight_gateway::{
    cluster::{Cluster, ShardScheme},
    Event, Intents,
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

    while let Some((_shard_id, event)) = events.next().await {
        tokio::spawn(handle_event(event, db.clone()));
    }
}

async fn handle_event(event: Event, db: MySqlPool) {
    if let Event::MessageCreate(msg) = event {
        if !msg.author.bot {
            // TODO edit sql to only allow user to gain xp one time per minute
            // This could be done using redis, but I don't really want to add more external requirements.
            // I plan to somehow do it within the DB, prob with an additional column.
            let xp_count = rand::thread_rng().gen_range(15..=25);
            if let Err(e) = sqlx::query(
                "INSERT INTO levels (id, xp) VALUES (?, ?) ON DUPLICATE KEY UPDATE xp=xp+?",
            )
            .bind(msg.author.id.to_string())
            .bind(xp_count)
            .bind(xp_count)
            .execute(&db)
            .await
            {
                eprintln!("SQL insert error: {e:?}");
            };
        }
    }
}
