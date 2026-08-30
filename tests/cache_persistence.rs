//! Integration coverage for persistent DNS cache lifecycle behavior.

mod common;

use common::TestDb;
use mydns::db;
use std::sync::Arc;

#[tokio::test]
async fn test_positive_cache_survives_pool_restart() {
    let test_db = TestDb::new();

    {
        let pool = test_db.init_pool().await;
        db::records::insert_cache(&pool, "restart.test.local.", "A", "10.1.2.3", 300, None)
            .await
            .unwrap();
        pool.close().await;
    }

    let pool = test_db.init_pool().await;
    let rows = db::records::get_cache(&pool, "restart.test.local.", "A")
        .await
        .unwrap();

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].value, "10.1.2.3");
    assert_eq!(rows[0].ttl, 300);
}

#[tokio::test]
async fn test_negative_cache_survives_pool_restart() {
    let test_db = TestDb::new();

    {
        let pool = test_db.init_pool().await;
        db::records::insert_cache(&pool, "missing.restart.test.local.", "A", "NX", 300, None)
            .await
            .unwrap();
        pool.close().await;
    }

    let pool = test_db.init_pool().await;
    let rows = db::records::get_cache(&pool, "missing.restart.test.local.", "A")
        .await
        .unwrap();

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].value, "NX");
}

#[tokio::test]
async fn test_expired_persistent_cache_is_hidden_and_pruned() {
    let test_db = TestDb::new();
    let pool = test_db.init_pool().await;

    db::records::insert_cache(&pool, "expired.test.local.", "A", "10.9.8.7", 0, None)
        .await
        .unwrap();

    let visible = db::records::get_cache(&pool, "expired.test.local.", "A")
        .await
        .unwrap();
    assert!(visible.is_empty(), "Expired entries must not be returned");

    let pruned = db::records::prune_cache(&pool).await.unwrap();
    assert_eq!(pruned, 1, "Expired entry should be physically pruned");

    let all = db::records::list_cache_entries(&pool).await.unwrap();
    assert!(all.is_empty());
}

#[tokio::test]
async fn test_persistent_cache_clear_removes_all_entries() {
    let test_db = TestDb::new();
    let pool = test_db.init_pool().await;

    db::records::insert_cache(&pool, "one.clear.test.local.", "A", "10.0.0.1", 300, None)
        .await
        .unwrap();
    db::records::insert_cache(&pool, "two.clear.test.local.", "A", "10.0.0.2", 300, None)
        .await
        .unwrap();

    assert_eq!(
        db::records::list_cache_entries(&pool).await.unwrap().len(),
        2
    );

    db::records::clear_cache(&pool).await.unwrap();

    assert!(db::records::list_cache_entries(&pool)
        .await
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn test_concurrent_cache_upserts_remain_deduplicated() {
    let test_db = TestDb::new();
    let pool = Arc::new(test_db.init_pool().await);
    let mut tasks = Vec::new();

    for ttl in 300..=331u32 {
        let pool = Arc::clone(&pool);
        tasks.push(tokio::spawn(async move {
            db::records::insert_cache(
                &pool,
                "concurrent.test.local.",
                "A",
                "10.20.30.40",
                ttl,
                None,
            )
            .await
        }));
    }

    for task in tasks {
        task.await.unwrap().unwrap();
    }

    let rows = db::records::get_cache(&pool, "concurrent.test.local.", "A")
        .await
        .unwrap();
    assert_eq!(
        rows.len(),
        1,
        "Concurrent identical writes must deduplicate"
    );
}
