#![warn(clippy::all, clippy::pedantic, clippy::nursery)]

use futures::stream::StreamExt;
use rand::Rng;
use sqlx::MySqlPool;
use std::{collections::HashSet, env, error::Error, sync::Arc};
use tokio::sync::Mutex;
use twilight_gateway::{
    cluster::{Cluster, ShardScheme},
    Event, Intents,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let token = env::var("DISCORD_TOKEN")?;
    let mysql = env::var("DATABASE_URL")?;

    let scheme = ShardScheme::Range {
        from: 0,
        to: 0,
        total: 1,
    };

    let intents = Intents::GUILD_MESSAGES;

    let (cluster, mut events) = Cluster::builder(token.clone(), intents)
        .shard_scheme(scheme)
        .build()
        .await?;

    let cluster = Arc::new(cluster);

    let cluster_spawn = cluster.clone();

    tokio::spawn(async move {
        cluster_spawn.up().await;
    });
    let users: UserList = Arc::new(Mutex::new(HashSet::new()));
    let db = sqlx::mysql::MySqlPoolOptions::new()
        .max_connections(50)
        .connect(&mysql)
        .await?;
    tokio::spawn(flush(db, users.clone()));
    while let Some((_shard_id, event)) = events.next().await {
        tokio::spawn(handle_event(event, users.clone()));
    }

    Ok(())
}

async fn handle_event(event: Event, users: UserList) {
    if let Event::MessageCreate(msg) = event {
        if msg.author.bot {
        } else {
            let mut u = users.lock().await;
            u.insert(msg.author.id.to_string());
        }
    }
}

type UserList = Arc<Mutex<HashSet<String>>>;

async fn flush(db: MySqlPool, users: UserList) {
    loop {
        let mut users_locked = users.lock().await;
        let actives = users_locked.clone();
        users_locked.clear();
        drop(users_locked);
        tokio::spawn(do_insert(db.clone(), actives));
        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
    }
}

async fn do_insert(db: MySqlPool, users: HashSet<String>) {
    for user in users {
        let db = db.clone();
        tokio::spawn(async move {
            let xp_count = rand::thread_rng().gen_range(15..=25);
            if let Err(e) = sqlx::query("INSERT INTO levels (id, xp) VALUES (?, ?) ON DUPLICATE KEY UPDATE levels SET xp=xp+? WHERE id = ?")
            .bind(&user)
            .bind(xp_count)
            .bind(xp_count)
            .bind(&user)
            .execute(&db)
            .await {
                eprintln!("SQL insert error: {e:?}");
            };
        });
    }
}
