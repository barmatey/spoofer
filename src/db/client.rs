use crate::db::errors::Error;
use crate::level2::create_level_updates_table;
use crate::shared::logger::Logger;
use crate::trade::create_trade_event_table;
use clickhouse::Client;
use tracing::Level;

async fn create_database(client: &Client, logger: &Logger, db_name: &str) -> Result<(), Error> {
    let query_check = format!(
        "SELECT name FROM system.databases WHERE name = '{}'",
        db_name
    );

    let existing: Vec<String> = client.query(&query_check).fetch_all().await?;

    if existing.is_empty() {
        logger.info(&format!(
            "Database '{}' does not exist. Creating...",
            db_name
        ));
        let query_create = format!("CREATE DATABASE {}", db_name);
        client.query(&query_create).execute().await?;
        logger.info(&format!("Database '{}' created", db_name));
    } else {
        logger.info(&format!("Database '{}' already exists", db_name));
    }

    Ok(())
}

async fn drop_all_tables(client: &Client, logger: &Logger, db_name: &str) -> Result<(), Error> {
    logger.info(&format!("Drop all tables in db {}", db_name));

    let tables: Vec<String> = client
        .query(&format!("SHOW TABLES FROM {}", db_name))
        .fetch_all()
        .await?;

    for table in tables {
        let drop_query = format!("DROP TABLE IF EXISTS {}.{}", db_name, table);
        logger.info(&format!("Dropping table {}", table));
        client.query(&drop_query).execute().await?;
    }

    Ok(())
}

async fn init_database(client: &Client, db_name: &str, recreate: bool) -> Result<(), Error> {
    let logger = Logger::new("initialisation", Level::INFO);
    create_database(client, &logger, db_name).await?;
    if recreate {
        drop_all_tables(client, &logger, db_name).await?;
    }
    create_level_updates_table(client, &logger, db_name).await?;
    create_trade_event_table(client, &logger, db_name).await?;
    logger.info("Successful database initialisation");
    Ok(())
}

pub struct ClickHouseClient {
    url: String,
    password: String,
    user: String,
    db_name: String,
}

impl ClickHouseClient {
    pub fn default() -> Self {
        Self {
            url: "".to_string(),
            password: "".to_string(),
            user: "".to_string(),
            db_name: "".to_string(),
        }
    }
    pub fn with_url(mut self, url: &str) -> Self {
        self.url = url.to_string();
        self
    }

    pub fn with_password(mut self, password: &str) -> Self {
        self.password = password.to_string();
        self
    }

    pub fn with_user(mut self, user: &str) -> Self {
        self.user = user.to_string();
        self
    }

    pub fn with_database(mut self, db_name: &str) -> Self {
        self.db_name = db_name.to_string();
        self
    }

    pub async fn build(self) -> Result<Client, Error> {
        let client = Client::default()
            .with_user(&self.user)
            .with_password(&self.password)
            .with_url(&self.url);
        init_database(&client, &self.db_name, true).await?;
        Ok(client.with_database(&self.db_name))
    }
}
