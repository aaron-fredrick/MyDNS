//! Integration coverage for the SQLite record, zone, settings, user, and cache APIs.

mod common;

use common::TestDb;
use mydns::db;
use mydns::db::records::{CreateRecord, UpdateRecord};

#[tokio::test]
async fn test_dns_record_crud_and_case_insensitive_lookup() {
    let test_db = TestDb::new();
    let pool = test_db.init_pool().await;

    let created = db::records::create_record(
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
    assert_eq!(created.name, "Host.Example.Local");

    let found = db::records::find_by_name(&pool, "host.example.local.")
        .await
        .unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].id, created.id);

    let updated = db::records::update_record(
        &pool,
        created.id,
        &UpdateRecord {
            name: None,
            record_type: Some("aaaa".into()),
            value: Some("2001:db8::10".into()),
            ttl: Some(600),
            priority: None,
        },
    )
    .await
    .unwrap()
    .unwrap();

    assert_eq!(updated.record_type, "AAAA");
    assert_eq!(updated.value, "2001:db8::10");
    assert_eq!(updated.ttl, 600);

    assert!(db::records::delete_record(&pool, created.id).await.unwrap());
    assert!(!db::records::delete_record(&pool, created.id).await.unwrap());
    assert!(db::records::get_record(&pool, created.id)
        .await
        .unwrap()
        .is_none());
}

#[tokio::test]
async fn test_dev_records_are_separated_from_authoritative_records() {
    let test_db = TestDb::new();
    let pool = test_db.init_pool().await;

    db::records::create_record(
        &pool,
        &CreateRecord {
            name: "authoritative.home.arpa".into(),
            record_type: "A".into(),
            value: "192.0.2.1".into(),
            ttl: 300,
            priority: None,
            is_dev: false,
        },
    )
    .await
    .unwrap();

    db::records::create_record(
        &pool,
        &CreateRecord {
            name: "temporary.example.com".into(),
            record_type: "A".into(),
            value: "192.0.2.2".into(),
            ttl: 60,
            priority: None,
            is_dev: true,
        },
    )
    .await
    .unwrap();

    assert_eq!(db::records::list_records(&pool).await.unwrap().len(), 1);
    assert_eq!(db::records::list_all_records(&pool).await.unwrap().len(), 2);

    assert_eq!(db::records::delete_dev_records(&pool).await.unwrap(), 1);
    assert_eq!(db::records::list_all_records(&pool).await.unwrap().len(), 1);
}

#[tokio::test]
async fn test_admin_seed_is_idempotent_and_lookup_works() {
    let test_db = TestDb::new();
    let pool = test_db.init_pool().await;

    db::users::seed_admin(&pool, "admin", "hash-1")
        .await
        .unwrap();
    db::users::seed_admin(&pool, "admin", "hash-2")
        .await
        .unwrap();

    assert_eq!(
        db::users::find_user_hash(&pool, "admin").await.unwrap(),
        Some("hash-1".into())
    );
    assert_eq!(
        db::users::find_user_hash(&pool, "missing").await.unwrap(),
        None
    );
}

#[tokio::test]
async fn test_zone_lifecycle_creates_and_removes_apex_records() {
    let test_db = TestDb::new();
    let pool = test_db.init_pool().await;

    let zone = db::zones::add_zone(&pool, "home.arpa").await.unwrap();
    assert_eq!(zone.name, "home.arpa");

    let zones = db::zones::list_zones(&pool).await.unwrap();
    assert_eq!(zones.len(), 1);
    assert_eq!(
        db::zones::list_zone_names(&pool).await.unwrap(),
        vec!["home.arpa"]
    );

    let apex = db::records::find_by_name(&pool, "home.arpa").await.unwrap();
    assert_eq!(apex.len(), 2);
    assert!(apex.iter().any(|r| r.record_type == "SOA"));
    assert!(apex.iter().any(|r| r.record_type == "NS"));

    db::records::create_record(
        &pool,
        &CreateRecord {
            name: "host.home.arpa".into(),
            record_type: "A".into(),
            value: "192.0.2.55".into(),
            ttl: 300,
            priority: None,
            is_dev: false,
        },
    )
    .await
    .unwrap();

    assert!(db::zones::remove_zone(&pool, "home.arpa").await.unwrap());
    assert!(db::zones::list_zones(&pool).await.unwrap().is_empty());
    assert!(db::records::find_by_name(&pool, "home.arpa")
        .await
        .unwrap()
        .is_empty());
    assert!(db::records::find_by_name(&pool, "host.home.arpa")
        .await
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn test_seed_zones_is_idempotent_and_normalizes_names() {
    let test_db = TestDb::new();
    let pool = test_db.init_pool().await;

    db::zones::seed_zones(
        &pool,
        &["Home.ARPA.".into(), "lab.local.".into(), "".into()],
    )
    .await
    .unwrap();
    db::zones::seed_zones(&pool, &["home.arpa".into(), "lab.local".into()])
        .await
        .unwrap();

    let names = db::zones::list_zone_names(&pool).await.unwrap();
    assert_eq!(names, vec!["home.arpa", "lab.local"]);
    assert_eq!(
        db::records::find_by_name(&pool, "home.arpa")
            .await
            .unwrap()
            .len(),
        2
    );
}

#[tokio::test]
async fn test_settings_are_inserted_and_replaced() {
    let test_db = TestDb::new();
    let pool = test_db.init_pool().await;

    assert_eq!(db::settings::get_setting(&pool, "resolver.mode").await.unwrap(), None);
    db::settings::set_setting(&pool, "resolver.mode", "forwarding")
        .await
        .unwrap();
    assert_eq!(
        db::settings::get_setting(&pool, "resolver.mode").await.unwrap(),
        Some("forwarding".into())
    );

    db::settings::set_setting(&pool, "resolver.mode", "recursive")
        .await
        .unwrap();
    assert_eq!(
        db::settings::get_setting(&pool, "resolver.mode").await.unwrap(),
        Some("recursive".into())
    );
}

#[tokio::test]
async fn test_cache_identity_keeps_distinct_values_and_updates_same_identity() {
    let test_db = TestDb::new();
    let pool = test_db.init_pool().await;

    db::cache::insert_cache(&pool, "multi.test.local", "A", "192.0.2.1", 300, None)
        .await
        .unwrap();
    db::cache::insert_cache(&pool, "multi.test.local", "A", "192.0.2.2", 300, None)
        .await
        .unwrap();
    db::cache::insert_cache(&pool, "multi.test.local", "a", "192.0.2.1", 600, None)
        .await
        .unwrap();

    let rows = db::cache::get_cache(&pool, "MULTI.TEST.LOCAL.", "A")
        .await
        .unwrap();
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().any(|r| r.value == "192.0.2.1" && r.ttl == 600));
    assert!(rows.iter().any(|r| r.value == "192.0.2.2" && r.ttl == 300));
}

#[tokio::test]
async fn test_cache_delete_by_name_clears_cname_dependents() {
    let test_db = TestDb::new();
    let pool = test_db.init_pool().await;

    for (name, value) in [
        ("alias.test.local", "target.test.local"),
        ("deep.test.local", "alias.test.local"),
    ] {
        db::records::create_record(
            &pool,
            &CreateRecord {
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

    for name in ["target.test.local", "alias.test.local", "deep.test.local"] {
        db::cache::insert_cache(&pool, name, "A", "192.0.2.9", 300, None)
            .await
            .unwrap();
    }

    let dependents = db::cache::find_cname_dependents(&pool, "target.test.local")
        .await
        .unwrap();
    assert_eq!(dependents.len(), 2);

    db::cache::delete_cache_for_name(&pool, "target.test.local")
        .await
        .unwrap();

    assert!(db::cache::get_cache(&pool, "target.test.local", "A")
        .await
        .unwrap()
        .is_empty());
    assert!(db::cache::get_cache(&pool, "alias.test.local", "A")
        .await
        .unwrap()
        .is_empty());
    assert!(db::cache::get_cache(&pool, "deep.test.local", "A")
        .await
        .unwrap()
        .is_empty());
}
