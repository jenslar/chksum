use std::{path::Path, time::{SystemTime, UNIX_EPOCH, SystemTimeError}};

use time::{format_description, OffsetDateTime, ext::NumericalDuration};

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

/// Converts `SystemTime` to `time::OffsetDateTime`.
fn systime2datetime(systime: SystemTime) -> Result<OffsetDateTime, SystemTimeError> {
    Ok(OffsetDateTime::UNIX_EPOCH + systime.duration_since(UNIX_EPOCH)?)
}

/// Converts seconds since Unix epoch start of January 1, 1970 to `time::OffsetDateTime`.
fn unixseconds2datetime(unix_seconds: i64) -> Result<OffsetDateTime, SystemTimeError> {
    Ok(OffsetDateTime::UNIX_EPOCH + unix_seconds.seconds())
}