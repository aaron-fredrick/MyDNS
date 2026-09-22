use sqlx::SqlitePool;
use std::path::PathBuf;
use tempfile::TempDir;

pub struct TestDb {
    pub temp_dir: TempDir,
    pub path: PathBuf,
}

impl TestDb {
    pub fn new() -> Self {
        let temp_dir = TempDir::new().expect("failed to create temporary DB directory");
        let path = temp_dir.path().join("test.db");
        Self { temp_dir, path }
    }

    pub fn path_str(&self) -> String {
        self.path.to_string_lossy().into_owned()
    }

    pub async fn init_pool(&self) -> SqlitePool {
        crate::mydns::db::init(&self.path_str())
            .await
            .expect("failed to initialize test database")
    }
}

impl Default for TestDb {
    fn default() -> Self {
        Self::new()
    }
}
