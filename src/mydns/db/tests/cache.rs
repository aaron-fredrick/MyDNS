use super::fixtures::TestDb;
use crate::mydns::db::records;
use chrono::Utc;

#[tokio::test]
async fn cache_insert_get_list_and_identity_rules_work() {
    let db = TestDb::new();
    let pool = db.init_pool().await;

    records::insert_cache(&pool, "Multi.Test.Local.", "A", "192.0.2.1", 300, None)
        .await
        .unwrap();
    records::insert_cache(&pool, "multi.test.local", "A", "192.0.2.2", 300, None)
        .await
        .unwrap();
    records::insert_cache(&pool, "multi.test.local", "a", "192.0.2.1", 600, None)
        .await
        .unwrap();
    records::insert_cache(
        &pool,
        "mx.test.local",
        "MX",
        "mail.test.local.",
        300,
        Some(10),
    )
    .await
    .unwrap();
    records::insert_cache(
        &pool,
        "mx.test.local",
        "MX",
        "mail.test.local.",
        300,
        Some(20),
    )
    .await
    .unwrap();

    let rows = records::get_cache(&pool, "MULTI.TEST.LOCAL.", "a")
        .await
        .unwrap();
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().any(|r| r.value == "192.0.2.1" && r.ttl == 600));
    assert!(rows.iter().any(|r| r.value == "192.0.2.2" && r.ttl == 300));

    let mx_rows = records::get_cache(&pool, "mx.test.local.", "mx")
        .await
        .unwrap();
    assert_eq!(mx_rows.len(), 2);

    let listed = records::list_cache_entries(&pool).await.unwrap();
    assert_eq!(listed.len(), 4);
}

#[tokio::test]
async fn cache_expiry_pruning_and_deletion_work() {
    let db = TestDb::new();
    let pool = db.init_pool().await;

    records::insert_cache(&pool, "live.example", "A", "192.0.2.1", 300, None)
        .await
        .unwrap();

    let expired_at = Utc::now().timestamp() - 1;
    sqlx::query(
        "INSERT INTO dns_cache (name, record_type, value, ttl, expires_at, priority)
         VALUES ('expired.example', 'A', '192.0.2.2', 60, ?, NULL)",
    )
    .bind(expired_at)
    .execute(&pool)
    .await
    .unwrap();

    assert!(records::get_cache(&pool, "expired.example", "A")
        .await
        .unwrap()
        .is_empty());
    assert_eq!(records::list_cache_entries(&pool).await.unwrap().len(), 1);

    assert_eq!(records::prune_cache(&pool).await.unwrap(), 1);
    assert_eq!(records::clear_cache(&pool).await.unwrap(), ());
    assert!(records::list_cache_entries(&pool).await.unwrap().is_empty());

    records::insert_cache(&pool, "delete.example", "A", "192.0.2.3", 300, None)
        .await
        .unwrap();
    records::delete_cache_entry(&pool, "DELETE.EXAMPLE.", "a")
        .await
        .unwrap();
    assert!(records::get_cache(&pool, "delete.example", "A")
        .await
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn cache_name_and_zone_deletion_respect_dns_boundaries() {
    let db = TestDb::new();
    let pool = db.init_pool().await;

    for name in [
        "example.com",
        "www.example.com",
        "deep.www.example.com",
        "notexample.com",
    ] {
        records::insert_cache(&pool, name, "A", "192.0.2.9", 300, None)
            .await
            .unwrap();
    }

    records::delete_cache_for_name(&pool, "www.example.com.")
        .await
        .unwrap();
    assert!(records::get_cache(&pool, "www.example.com", "A")
        .await
        .unwrap()
        .is_empty());
    assert!(!records::get_cache(&pool, "deep.www.example.com", "A")
        .await
        .unwrap()
        .is_empty());
    assert!(!records::get_cache(&pool, "example.com", "A")
        .await
        .unwrap()
        .is_empty());
    assert!(!records::get_cache(&pool, "notexample.com", "A")
        .await
        .unwrap()
        .is_empty());

    records::delete_cache_for_zone(&pool, "example.com.")
        .await
        .unwrap();
    assert!(records::get_cache(&pool, "example.com", "A")
        .await
        .unwrap()
        .is_empty());
    assert!(records::get_cache(&pool, "deep.www.example.com", "A")
        .await
        .unwrap()
        .is_empty());
    assert!(!records::get_cache(&pool, "notexample.com", "A")
        .await
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn cname_dependents_are_recursive_and_cycle_safe() {
    let db = TestDb::new();
    let pool = db.init_pool().await;

    for (name, value) in [
        ("alias.example", "target.example"),
        ("deep.example", "alias.example"),
        ("cycle-a.example", "cycle-b.example"),
        ("cycle-b.example", "cycle-a.example"),
    ] {
        records::create_record(
            &pool,
            &records::CreateRecord {
                name: name.into(),
                record_type: "CNAME".into(),
                value: value.into(),
                ttl: 300,
                priority: None,
                is_dev: false,
            },
        )
        .await
        .unwrap();
    }

    let dependents = records::find_cname_dependents(&pool, "TARGET.EXAMPLE.")
        .await
        .unwrap();
    assert_eq!(dependents.len(), 2);
    assert!(dependents.contains(&"alias.example".to_string()));
    assert!(dependents.contains(&"deep.example".to_string()));

    records::insert_cache(&pool, "target.example", "A", "192.0.2.10", 300, None)
        .await
        .unwrap();
    records::insert_cache(&pool, "alias.example", "A", "192.0.2.10", 300, None)
        .await
        .unwrap();
    records::insert_cache(&pool, "deep.example", "A", "192.0.2.10", 300, None)
        .await
        .unwrap();

    records::delete_cache_for_name(&pool, "target.example")
        .await
        .unwrap();
    assert!(records::get_cache(&pool, "target.example", "A")
        .await
        .unwrap()
        .is_empty());
    assert!(records::get_cache(&pool, "alias.example", "A")
        .await
        .unwrap()
        .is_empty());
    assert!(records::get_cache(&pool, "deep.example", "A")
        .await
        .unwrap()
        .is_empty());

    let cycle_dependents = records::find_cname_dependents(&pool, "cycle-a.example")
        .await
        .unwrap();
    assert_eq!(cycle_dependents.len(), 2);
}
