use super::fixtures::TestDb;
use crate::mydns::db::blocklist::{self, CreateBlocklistEntry, UpdateBlocklistEntry};

#[tokio::test]
async fn blocklist_crud_normalizes_filters_updates_and_deletes() {
    let db = TestDb::new();
    let pool = db.init_pool().await;

    let first = blocklist::create_entry(
        &pool,
        &CreateBlocklistEntry {
            domain: "Ads.Example.COM.".into(),
            enabled: true,
            source: "manual".into(),
            reason: Some("ads".into()),
        },
    )
    .await
    .unwrap();

    let second = blocklist::create_entry(
        &pool,
        &CreateBlocklistEntry {
            domain: "tracker.example.org".into(),
            enabled: false,
            source: "imported".into(),
            reason: None,
        },
    )
    .await
    .unwrap();

    assert_eq!(first.domain, "ads.example.com");
    assert_eq!(first.source, "manual");
    assert!(first.enabled);
    assert_eq!(second.source, "imported");

    let listed = blocklist::list_entries(&pool).await.unwrap();
    assert_eq!(listed.len(), 2);
    assert_eq!(listed[0].domain, "ads.example.com");

    assert_eq!(
        blocklist::list_enabled_domains(&pool).await.unwrap(),
        vec!["ads.example.com"]
    );

    let fetched = blocklist::get_entry(&pool, first.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(fetched.id, first.id);

    let updated = blocklist::update_entry(
        &pool,
        second.id,
        &UpdateBlocklistEntry {
            enabled: Some(true),
            reason: Some("tracking".into()),
        },
    )
    .await
    .unwrap()
    .unwrap();
    assert!(updated.enabled);
    assert_eq!(updated.reason.as_deref(), Some("tracking"));

    let unchanged = blocklist::update_entry(
        &pool,
        second.id,
        &UpdateBlocklistEntry {
            enabled: None,
            reason: None,
        },
    )
    .await
    .unwrap()
    .unwrap();
    assert!(unchanged.enabled);
    assert_eq!(unchanged.reason.as_deref(), Some("tracking"));

    assert_eq!(
        blocklist::list_enabled_domains(&pool).await.unwrap(),
        vec!["ads.example.com", "tracker.example.org"]
    );

    assert!(blocklist::delete_entry(&pool, first.id).await.unwrap());
    assert!(!blocklist::delete_entry(&pool, first.id).await.unwrap());
    assert!(blocklist::get_entry(&pool, first.id)
        .await
        .unwrap()
        .is_none());
    assert!(blocklist::get_entry(&pool, 9999).await.unwrap().is_none());
}

#[tokio::test]
async fn blocklist_rejects_invalid_sources_and_duplicate_domains() {
    let db = TestDb::new();
    let pool = db.init_pool().await;

    let invalid = blocklist::create_entry(
        &pool,
        &CreateBlocklistEntry {
            domain: "bad-source.example".into(),
            enabled: true,
            source: "unknown".into(),
            reason: None,
        },
    )
    .await;
    assert!(invalid.is_err());

    blocklist::create_entry(
        &pool,
        &CreateBlocklistEntry {
            domain: "duplicate.example".into(),
            enabled: true,
            source: "manual".into(),
            reason: None,
        },
    )
    .await
    .unwrap();

    let duplicate = blocklist::create_entry(
        &pool,
        &CreateBlocklistEntry {
            domain: "DUPLICATE.EXAMPLE.".into(),
            enabled: true,
            source: "manual".into(),
            reason: None,
        },
    )
    .await;
    assert!(duplicate.is_err());

    assert!(blocklist::update_entry(
        &pool,
        9999,
        &UpdateBlocklistEntry {
            enabled: Some(true),
            reason: Some("missing".into()),
        },
    )
    .await
    .unwrap()
    .is_none());
}

#[test]
fn serde_defaults_for_blocklist_create_payload() {
    let req: CreateBlocklistEntry =
        serde_json::from_str(r#"{ "domain": "ads.example.com", "reason": null }"#).unwrap();
    assert!(req.enabled);
    assert_eq!(req.source, "manual");
}
