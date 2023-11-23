use std::path::Path;

use time::{format_description, OffsetDateTime};

/// Formats datetime to string, `YYYY-MM-DD HH:mm:SS.fff`.
pub fn datetime_to_string(datetime: &OffsetDateTime) -> String {
    let format = format_description::parse("[year]-[month]-[day] [hour]:[minute]:[second].[subsecond digits:3]")
        .expect("Failed to parse xatetime format");
    datetime.format(&format)
        .expect("Failed to format datetime string")
}

pub fn now_to_string() -> std::string::String {
    datetime_to_string(&OffsetDateTime::now_utc())
}

/// Returns modified datetime for path.
pub fn datetime_modified(path: &Path) -> Option<String> {
    match path.metadata() {
        Ok(m) => {
            let dt: OffsetDateTime = m.modified().ok()?.into();
            Some(datetime_to_string(&dt))
        },
        Err(_) => None
    }
}