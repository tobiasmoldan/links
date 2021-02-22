pub mod db {
    use sqlx::FromRow;

    #[derive(Debug, FromRow)]
    pub struct Entry {
        pub path: String,
        pub url: String,
        pub created: String,
    }
}

pub mod http {
    use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
    use serde::{Deserialize, Serialize};
    #[derive(Debug, Deserialize)]
    pub struct NewEntryRequest {
        pub path: String,
        pub url: String,
    }
    #[derive(Debug, Serialize)]
    pub struct EntryResponse {
        path: String,
        url: String,
        created: DateTime<Utc>,
    }

    impl From<super::db::Entry> for EntryResponse {
        fn from(entry: super::db::Entry) -> Self {
            let date = NaiveDateTime::parse_from_str(&entry.created, "%Y-%m-%d %H:%M:%S")
                .unwrap_or_else(|_| NaiveDate::from_ymd(0, 1, 1).and_hms(0, 0, 0));

            EntryResponse {
                created: DateTime::from_utc(date, Utc),
                path: entry.path,
                url: entry.url,
            }
        }
    }
}
