use super::fixtures::TestDb;
use crate::mydns::db::settings;

#[tokio::test]
async fn get_setting_returns_none_for_missing_key() {
    let db = TestDb::new();
    let pool = db.init_pool().await;

    assert_eq!(settings::get_setting(&pool, "missing").await.unwrap(), None);
}

#[tokio::test]
async fn set_setting_inserts_then_replaces_value() {
    let db = TestDb::new();
    let pool = db.init_pool().await;

    settings::set_setting(&pool, "resolver.mode", "forwarding")
        .await
        .unwrap();
    assert_eq!(
        settings::get_setting(&pool, "resolver.mode").await.unwrap(),
        Some("forwarding".to_string())
    );

    settings::set_setting(&pool, "resolver.mode", "recursive")
        .await
        .unwrap();
    assert_eq!(
        settings::get_setting(&pool, "resolver.mode").await.unwrap(),
        Some("recursive".to_string())
    );
}
