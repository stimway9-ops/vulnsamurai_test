use mongodb::{Client, Database};
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<Database>,
    pub jwt_secret: String,
}

impl AppState {
    pub async fn init() -> anyhow::Result<Self> {
        let mongo_uri = std::env::var("MONGO_URI")
            .unwrap_or_else(|_| "mongodb://mongo:27017".into());
        let db_name = std::env::var("MONGO_DB")
            .unwrap_or_else(|_| "vulnsamurai".into());
        let jwt_secret = std::env::var("JWT_SECRET")
            .unwrap_or_else(|_| "change_me_in_production".into());

        let client = Client::with_uri_str(&mongo_uri).await?;
        let db = client.database(&db_name);

        // Ping to verify connection
        db.run_command(bson::doc! { "ping": 1 }, None).await?;

        Ok(Self {
            db: Arc::new(db),
            jwt_secret,
        })
    }
}
