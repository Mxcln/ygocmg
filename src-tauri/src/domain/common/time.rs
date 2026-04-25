pub type AppTimestamp = chrono::DateTime<chrono::Utc>;

pub fn now_utc() -> AppTimestamp {
    chrono::Utc::now()
}
