use chrono::NaiveDate;
use url::form_urlencoded;

use crate::{errors::RequestError, models::DateRange};

const DEFAULT_LIMIT: i64 = 20;
const MAX_LIMIT: i64 = 100;
const MAX_RANGE_SPAN_DAYS: i64 = 365;
pub(crate) const DIMENSIONS: [&str; 11] = [
    "language",
    "timezone",
    "utm_source",
    "utm_medium",
    "utm_campaign",
    "utm_term",
    "utm_content",
    "referrer_host",
    "device",
    "browser",
    "os",
];

pub(crate) fn parse_range(from: &str, to: &str) -> Result<DateRange, RequestError> {
    let from_date = parse_date(from)?;
    let to_date = parse_date(to)?;
    if from_date > to_date {
        return Err(RequestError::InvalidDateRange(
            "from must be before or equal to to",
        ));
    }
    if to_date.signed_duration_since(from_date).num_days() > MAX_RANGE_SPAN_DAYS {
        return Err(RequestError::DateRangeTooLarge);
    }
    Ok(DateRange {
        from: from_date,
        to: to_date,
    })
}

fn parse_date(value: &str) -> Result<NaiveDate, RequestError> {
    let date = NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .map_err(|_| RequestError::InvalidDateRange("from and to must use YYYY-MM-DD"))?;
    if date.format("%Y-%m-%d").to_string() != value {
        return Err(RequestError::InvalidDateRange(
            "from and to must use YYYY-MM-DD",
        ));
    }
    Ok(date)
}

pub(crate) fn parse_limit(value: Option<&str>) -> Result<i64, RequestError> {
    let limit = value.map_or(Ok(DEFAULT_LIMIT), |value| {
        value.parse::<i64>().map_err(|_| RequestError::InvalidLimit)
    })?;
    if !(1..=MAX_LIMIT).contains(&limit) {
        return Err(RequestError::InvalidLimit);
    }
    Ok(limit)
}

pub(crate) fn parse_limit_query(query: Option<&str>) -> Result<i64, RequestError> {
    let mut limit = None;
    for (key, value) in form_urlencoded::parse(query.unwrap_or_default().as_bytes()) {
        if key == "limit" {
            if limit.is_some() {
                return Err(RequestError::InvalidLimit);
            }
            limit = Some(value.into_owned());
        }
    }
    parse_limit(limit.as_deref())
}

pub(crate) fn parse_events_query(
    query: Option<&str>,
) -> Result<(i64, Option<String>), RequestError> {
    let mut limit = None;
    let mut event_name = None;
    for (key, value) in form_urlencoded::parse(query.unwrap_or_default().as_bytes()) {
        match key.as_ref() {
            "limit" => {
                if limit.is_some() {
                    return Err(RequestError::InvalidLimit);
                }
                limit = Some(value.into_owned());
            }
            "event_name" => {
                if event_name.is_some() {
                    return Err(RequestError::InvalidEventName);
                }
                event_name = Some(value.into_owned());
            }
            _ => {}
        }
    }
    let limit = parse_limit(limit.as_deref())?;
    if event_name
        .as_deref()
        .is_some_and(|name| !valid_event_name(name))
    {
        return Err(RequestError::InvalidEventName);
    }
    Ok((limit, event_name))
}

fn valid_event_name(name: &str) -> bool {
    let mut bytes = name.bytes();
    bytes.next().is_some_and(|b| b.is_ascii_alphabetic())
        && bytes.all(|b| b.is_ascii_alphanumeric() || b"_.-".contains(&b))
        && name.len() <= 64
}

pub(crate) fn validate_dimension(value: &str) -> Result<(), RequestError> {
    DIMENSIONS
        .contains(&value)
        .then_some(())
        .ok_or(RequestError::InvalidDimension)
}

#[cfg(test)]
mod tests {
    use super::{parse_events_query, parse_limit_query, parse_range};

    #[test]
    fn validates_inclusive_366_day_range() {
        assert!(parse_range("2026-01-01", "2027-01-01").is_ok());
        assert!(parse_range("2026-01-01", "2027-01-02").is_err());
    }

    #[test]
    fn rejects_invalid_date_ranges() {
        assert!(parse_range("2026-02-30", "2026-03-01").is_err());
        assert!(parse_range("2026-03-02", "2026-03-01").is_err());
        assert!(parse_range("2026-1-01", "2026-01-02").is_err());
    }

    #[test]
    fn validates_custom_event_filter_and_limit() {
        assert_eq!(parse_events_query(None).unwrap(), (20, None));
        assert_eq!(
            parse_events_query(Some("event_name=checkout_started&limit=1")).unwrap(),
            (1, Some("checkout_started".to_owned()))
        );
        assert!(parse_events_query(Some("event_name=1invalid")).is_err());
        assert!(parse_events_query(Some("event_name=bad%20name")).is_err());
        assert!(parse_events_query(Some("event_name=a&event_name=b")).is_err());
        assert!(parse_events_query(Some("limit=101")).is_err());
    }

    #[test]
    fn validates_limit() {
        assert_eq!(parse_limit_query(None).unwrap(), 20);
        assert_eq!(parse_limit_query(Some("limit=100")).unwrap(), 100);
        assert!(parse_limit_query(Some("limit=0")).is_err());
        assert!(parse_limit_query(Some("limit=101")).is_err());
        assert!(parse_limit_query(Some("limit=nope")).is_err());
        assert!(parse_limit_query(Some("limit=1&limit=2")).is_err());
    }
}
