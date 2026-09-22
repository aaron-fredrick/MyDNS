use super::fixtures::TestDb;
use crate::mydns::db::records::{self, CreateRecord, UpdateRecord};

#[tokio::test]
async fn record_crud_covers_case_normalization_and_partial_updates() {
    let db = TestDb::new();
    let pool = db.init_pool().await;

    let created = records::create_record(
        &pool,
        &CreateRecord {
            name: "Host.Example.Local".into(),
            record_type: "a".into(),
            value: "192.0.2.10".into(),
            ttl: 300,
            priority: None,
            is_dev: false,
        },
    )
    .await
    .unwrap();
    assert_eq!(created.record_type, "A");
    assert!(!created.is_dev);

    let found = records::find_by_name(&pool, "host.example.local.")
        .await
        .unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].id, created.id);

    let updated = records::update_record(
        &pool,
        created.id,
        &UpdateRecord {
            name: Some("Mail.Example.Local.".into()),
            record_type: Some("mx".into()),
            value: Some("mail.example.local.".into()),
            ttl: Some(600),
            priority: Some(10),
        },
    )
    .await
    .unwrap()
    .unwrap();

    assert_eq!(updated.name, "Mail.Example.Local.");
    assert_eq!(updated.record_type, "MX");
    assert_eq!(updated.value, "mail.example.local.");
    assert_eq!(updated.ttl, 600);
    assert_eq!(updated.priority, Some(10));

    let unchanged = records::update_record(
        &pool,
        created.id,
        &UpdateRecord {
            name: None,
            record_type: None,
            value: None,
            ttl: None,
            priority: None,
        },
    )
    .await
    .unwrap()
    .unwrap();
    assert_eq!(unchanged.name, updated.name);
    assert_eq!(unchanged.record_type, updated.record_type);

    assert!(records::update_record(
        &pool,
        9999,
        &UpdateRecord {
            name: None,
            record_type: Some("A".into()),
            value: None,
            ttl: None,
            priority: None,
        },
    )
    .await
    .unwrap()
    .is_none());

    assert!(records::delete_record(&pool, created.id).await.unwrap());
    assert!(!records::delete_record(&pool, created.id).await.unwrap());
    assert!(records::get_record(&pool, created.id)
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn list_records_excludes_dev_records_but_all_records_includes_them() {
    let db = TestDb::new();
    let pool = db.init_pool().await;

    for (name, is_dev) in [
        ("authoritative.example", false),
        ("temporary.example", true),
    ] {
        records::create_record(
            &pool,
            &CreateRecord {
                name: name.into(),
                record_type: "A".into(),
                value: "192.0.2.1".into(),
                ttl: 60,
                priority: None,
                is_dev,
            },
        )
        .await
        .unwrap();
    }

    assert_eq!(records::list_records(&pool).await.unwrap().len(), 1);
    assert_eq!(records::list_all_records(&pool).await.unwrap().len(), 2);
    assert_eq!(records::delete_dev_records(&pool).await.unwrap(), 1);
    assert_eq!(records::list_all_records(&pool).await.unwrap().len(), 1);
    assert_eq!(records::delete_dev_records(&pool).await.unwrap(), 0);
}

#[tokio::test]
async fn users_can_be_seeded_idempotently_and_missing_users_return_none() {
    let db = TestDb::new();
    let pool = db.init_pool().await;

    records::seed_admin(&pool, "admin", "hash-1").await.unwrap();
    records::seed_admin(&pool, "admin", "hash-2").await.unwrap();

    assert_eq!(
        records::find_user_hash(&pool, "admin").await.unwrap(),
        Some("hash-1".into())
    );
    assert_eq!(
        records::find_user_hash(&pool, "missing").await.unwrap(),
        None
    );
}

#[tokio::test]
async fn zone_seed_add_remove_and_apex_creation_are_idempotent() {
    let db = TestDb::new();
    let pool = db.init_pool().await;

    records::seed_zones(
        &pool,
        &[
            "Home.ARPA.".into(),
            ".".into(),
            "".into(),
            "Lab.Local.".into(),
        ],
    )
    .await
    .unwrap();
    records::seed_zones(&pool, &["home.arpa".into(), "lab.local".into()])
        .await
        .unwrap();

    assert_eq!(
        records::list_zone_names(&pool).await.unwrap(),
        vec![".", "home.arpa", "lab.local"]
    );

    let home_apex = records::find_by_name(&pool, "HOME.ARPA.").await.unwrap();
    assert_eq!(home_apex.len(), 2);
    assert!(home_apex.iter().any(|r| r.record_type == "SOA"));
    assert!(home_apex.iter().any(|r| r.record_type == "NS"));

    let root_apex = records::find_by_name(&pool, ".").await.unwrap();
    assert_eq!(root_apex.len(), 2);

    records::create_apex_soa_and_ns(&pool, "HOME.ARPA.")
        .await
        .unwrap();
    assert_eq!(
        records::find_by_name(&pool, "home.arpa")
            .await
            .unwrap()
            .len(),
        2
    );

    let added = records::add_zone(&pool, "example.com").await.unwrap();
    assert_eq!(added.name, "example.com");
    assert_eq!(
        records::find_by_name(&pool, "example.com")
            .await
            .unwrap()
            .len(),
        2
    );
    assert!(records::add_zone(&pool, "example.com").await.is_err());

    records::create_record(
        &pool,
        &CreateRecord {
            name: "host.example.com".into(),
            record_type: "A".into(),
            value: "192.0.2.55".into(),
            ttl: 300,
            priority: None,
            is_dev: false,
        },
    )
    .await
    .unwrap();

    assert!(records::remove_zone(&pool, "example.com").await.unwrap());
    assert!(!records::remove_zone(&pool, "example.com").await.unwrap());
    assert!(records::find_by_name(&pool, "example.com")
        .await
        .unwrap()
        .is_empty());
    assert!(records::find_by_name(&pool, "host.example.com")
        .await
        .unwrap()
        .is_empty());
}
