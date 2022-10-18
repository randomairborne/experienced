use futures::stream::StreamExt;
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
    let mysql = env::var("MYSQL_URL")?;

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
    match event {
        Event::MessageCreate(msg) => {
            if msg.author.bot {
                return;
            } else {
                let mut u = users.lock().await;
                u.insert(msg.author.id.to_string());
            }
        }
        _ => {}
    };
}

type UserList = Arc<Mutex<HashSet<String>>>;

async fn flush(db: MySqlPool, users: UserList) {
    loop {
        tokio::time::sleep(std::time::Duration::from_secs(60)).await;
        query!("INSERT INTO levels VALUE")
    }
}
