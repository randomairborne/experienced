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

#[sqlx::test(migrations = "../migrations/")]
async fn audit_log_refetch(db: PgPool) -> Result<(), Box<dyn std::error::Error>> {
    let original_event = AuditLogEvent {
        guild_id: Id::new(1),
        user_id: Id::new(2),
        moderator: Id::new(3),
        timestamp: 50,
        previous: 100,
        delta: -100,
        reset: true,
        set: false,
    };
    add_audit_log_event(&db, original_event).await?;
    let roundtripped_event = get_audit_log_events(&db, Id::new(1), None, None).await?;
    assert_eq!(roundtripped_event, &[original_event]);
    Ok(())
}

#[sqlx::test(migrations = "../migrations/")]
async fn audit_log_multi(db: PgPool) -> Result<(), Box<dyn std::error::Error>> {
    let original_events = vec![
        AuditLogEvent {
            guild_id: Id::new(1),
            user_id: Id::new(2),
            moderator: Id::new(3),
            timestamp: 50,
            previous: 100,
            delta: -100,
            reset: true,
            set: false,
        },
        AuditLogEvent {
            guild_id: Id::new(1),
            user_id: Id::new(4),
            moderator: Id::new(5),
            timestamp: 591,
            previous: 15,
            delta: 50,
            reset: false,
            set: true,
        },
    ];
    for event in &original_events {
        add_audit_log_event(&db, *event).await?;
    }
    let roundtripped_events = get_audit_log_events(&db, Id::new(1), None, None).await?;

    assert_eq!(
        roundtripped_events.sorted_by_timestamp(),
        original_events.sorted_by_timestamp()
    );
    Ok(())
}

#[sqlx::test(migrations = "../migrations/")]
async fn audit_logs_deleted_guild(db: PgPool) -> Result<(), Box<dyn std::error::Error>> {
    let original_events = vec![
        AuditLogEvent {
            guild_id: Id::new(1),
            user_id: Id::new(2),
            moderator: Id::new(3),
            timestamp: 50,
            previous: 100,
            delta: -100,
            reset: false,
            set: false,
        },
        AuditLogEvent {
            guild_id: Id::new(1),
            user_id: Id::new(4),
            moderator: Id::new(5),
            timestamp: 591,
            previous: 15,
            delta: 50,
            reset: false,
            set: false,
        },
        AuditLogEvent {
            guild_id: Id::new(2),
            user_id: Id::new(4),
            moderator: Id::new(5),
            timestamp: 595,
            previous: 100,
            delta: 50,
            reset: false,
            set: true,
        },
    ];
    for event in &original_events {
        add_audit_log_event(&db, *event).await?;
    }
    delete_audit_log_events_guild(&db, Id::new(1)).await?;

    let should_be_nonexistent = get_audit_log_events(&db, Id::new(1), None, None).await?;
    let roundtripped_events = get_audit_log_events(&db, Id::new(2), None, None).await?;

    assert!(should_be_nonexistent.is_empty());
    assert_eq!(
        roundtripped_events.sorted_by_timestamp(),
        vec![original_events[2]]
    );
    Ok(())
}

trait SortedByTimestamp {
    fn sorted_by_timestamp(self) -> Self;
}

impl SortedByTimestamp for Vec<AuditLogEvent> {
    fn sorted_by_timestamp(mut self) -> Self {
        self.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
        self
    }
}
