use evaluate_database::EvaluateDatabase;
use github_handler_storage::GithubHanlderStorage;
use init::database_connection;
use std::{path::PathBuf, sync::Arc};

pub mod evaluate_database;
pub mod github_handler_storage;
pub mod init;

#[derive(Clone)]
pub struct Context {
    pub services: Arc<Service>,
    pub base_dir: PathBuf,
}

impl Context {
    pub async fn new(db_url: &str, base_dir: PathBuf) -> Self {
        Context {
            services: Service::shared(db_url).await,
            base_dir,
        }
    }

    pub fn github_handler_stg(&self) -> GithubHanlderStorage {
        self.services.github_handler_storage.clone()
    }

    pub fn evaluate_database_stg(&self) -> EvaluateDatabase {
        self.services.evaluate_database.clone()
    }
}
#[derive(Clone)]
pub struct Service {
    github_handler_storage: GithubHanlderStorage,
    evaluate_database: EvaluateDatabase,
}

impl Service {
    async fn new(db_url: &str) -> Self {
        let connection = Arc::new(database_connection(db_url).await.unwrap());
        Self {
            github_handler_storage: GithubHanlderStorage::new(connection.clone()).await,
            evaluate_database: EvaluateDatabase::new(connection.clone()).await,
        }
    }

    async fn shared(db_url: &str) -> Arc<Self> {
        Arc::new(Self::new(db_url).await)
    }
}
