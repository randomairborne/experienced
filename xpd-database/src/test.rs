use sqlx::PgPool;
use twilight_model::id::Id;

use crate::*;
#[sqlx::test(migrations = "../migrations/")]
async fn find_deletes_returns_correctly(db: PgPool) -> Result<(), Box<dyn std::error::Error>> {
    // Added 31 days ago, should be included
    query!(
        "INSERT INTO guild_cleanups (guild, removed_at) VALUES (1, NOW() - interval '31 days')
        ON CONFLICT (guild) DO UPDATE SET removed_at = excluded.removed_at"
    )
    .execute(&db)
    .await?;
    // Added 29 days ago, should NOT be included
    query!(
        "INSERT INTO guild_cleanups (guild, removed_at) VALUES (2, NOW() - interval '29 days')
        ON CONFLICT (guild) DO UPDATE SET removed_at = excluded.removed_at"
    )
    .execute(&db)
    .await?;

    // Added 31 days ago, but will be deleted
    query!(
        "INSERT INTO guild_cleanups (guild, removed_at) VALUES (3, NOW() - interval '31 days')
        ON CONFLICT (guild) DO UPDATE SET removed_at = excluded.removed_at"
    )
    .execute(&db)
    .await?;
    delete_guild_cleanup(&db, Id::new(3)).await?;
    let cleanups = get_active_guild_cleanups(&db).await?;
    assert!(cleanups.contains(&Id::new(1)));
    assert!(!cleanups.contains(&Id::new(2)));
    Ok(())
}
