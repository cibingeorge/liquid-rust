use std::convert::TryInto;
use std::fmt;
use std::ops;

mod strftime;
use chrono::Timelike;
use chrono::Datelike;
use chrono_tz::OffsetComponents;
use time::UtcOffset;
use chrono::TimeZone;

use super::Date;

/// Liquid's native date + time type.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(transparent)]
#[repr(transparent)]
pub struct DateTime {
    #[serde(with = "friendly_date_time")]
    inner: DateTimeImpl,
}

type DateTimeImpl = time::OffsetDateTime;

impl DateTime {
    /// Create a `DateTime` from the current moment.
    pub fn now() -> Self {
        Self {
            //inner: DateTimeImpl::now_local().unwrap_or_else(|_| DateTimeImpl::now_utc()),
            inner: DateTimeImpl::now_utc(),        }
    }


    /// Create a `DateTime` from the inner implementation
    pub fn from_chrono_datetime(dt: chrono::DateTime<chrono::FixedOffset>) -> Self {
        let date = time::Date::from_calendar_date(dt.year(), time::Month::try_from(dt.month() as u8).unwrap(), dt.day() as u8).unwrap();
        let time = time::Time::from_hms_nano(dt.hour() as u8, dt.minute() as u8, dt.second() as u8, dt.nanosecond()).unwrap();
        let offset = time::UtcOffset::from_whole_seconds(dt.offset().local_minus_utc()).unwrap();

        let inner = time::OffsetDateTime::new_in_offset(date, time, offset);

        Self {
            inner
        }
    }

    /// Makes a new NaiveDate from the calendar date (year, month and day).
    ///
    /// Panics on the out-of-range date, invalid month and/or day.
    pub fn from_ymd(year: i32, month: u8, day: u8) -> Self {
        Self {
            inner: time::Date::from_calendar_date(
                year,
                month.try_into().expect("the month is out of range"),
                day,
            )
            .expect("one or more components were invalid")
            .with_hms(0, 0, 0)
            .expect("one or more components were invalid")
            .assume_offset(time::macros::offset!(UTC)),
        }
    }

    /// Makes a new NaiveDate from the calendar date (year, month and day) at specific offset.
    ///
    /// Panics on the out-of-range date, invalid month and/or day.
    pub fn from_date(date: &Date) -> Option<Self> {
        let tz = rb_date_parser::get_current_timezone();
        let local_time = tz.with_ymd_and_hms(date.year().into(), date.month().into(), date.day().into(), 0, 0, 0).earliest();
        let offset = local_time
            .and_then(|x| Some(x.fixed_offset().offset().local_minus_utc()))
            .and_then(|offset_in_sec| time::UtcOffset::from_whole_seconds(offset_in_sec).ok());

        if offset.is_none() {
            return None;
        }
        let offset = offset.unwrap();

        let month = date.month().try_into().expect("the month is out of range");
        time::Date::from_calendar_date(
            date.year(),
            month,
            date.day(),
        ).and_then(|dt| {
            dt.with_hms(0, 0, 0)
        })
        .and_then(|dt| {
            Ok(Self { inner: dt.assume_offset(offset)})
        }).ok()
    }

    /// Convert a `str` to `Self`
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(other: &str) -> Option<Self> {
        parse_date_time(other).map(|d| Self { inner: d })
    }

    /// Convert a number to `Self`
    pub fn from_unix_timestamp(ts: i64) -> Option<Self> {
        let res = DateTimeImpl::from_unix_timestamp(ts).map(|d| {
            let tz = rb_date_parser::get_current_timezone();
            let local_time = tz.timestamp_opt(ts, 0).earliest().unwrap();
            let offset_in_sec = local_time.fixed_offset().offset().local_minus_utc();

            let offset = UtcOffset::from_whole_seconds(offset_in_sec).unwrap();
            Self{inner: d.to_offset(offset)}
        }).ok();
        res
    }

    /// Replace date with `other`.
    pub fn with_date(self, other: Date) -> Self {
        Self {
            inner: self.inner.replace_date(other.inner),
        }
    }


    /// Changes the associated time zone. This does not change the actual DateTime (but will change the string representation).
    pub fn with_offset(self, offset: time::UtcOffset) -> Self {
        Self {
            inner: self.inner.to_offset(offset),
        }
    }

    /// Retrieves a date component.
    pub fn date(self) -> Date {
        Date {
            inner: self.inner.date(),
        }
    }

    /// Formats the combined date and time with the specified format string.
    ///
    /// See the [chrono::format::strftime](https://docs.rs/chrono/latest/chrono/format/strftime/index.html)
    /// module on the supported escape sequences.
    #[inline]
    pub fn format(&self, fmt: &str) -> Result<String, strftime::DateFormatError> {
        strftime::strftime(self.inner, fmt)
    }

    /// Returns an RFC 2822 date and time string such as `Tue, 1 Jul 2003 10:52:37 +0200`.
    pub fn to_rfc2822(&self) -> String {
        self.inner
            .format(&time::format_description::well_known::Rfc2822)
            .expect("always valid")
    }
}

impl DateTime {
    /// Get the year of the date.
    #[inline]
    pub fn year(&self) -> i32 {
        self.inner.year()
    }
    /// Get the month.
    #[inline]
    pub fn month(&self) -> u8 {
        self.inner.month() as u8
    }
    /// Get the day of the month.
    ///
    //// The returned value will always be in the range 1..=31.
    #[inline]
    pub fn day(&self) -> u8 {
        self.inner.day()
    }
    /// Get the day of the year.
    ///
    /// The returned value will always be in the range 1..=366 (1..=365 for common years).
    #[inline]
    pub fn ordinal(&self) -> u16 {
        self.inner.ordinal()
    }
    /// Get the ISO week number.
    ///
    /// The returned value will always be in the range 1..=53.
    #[inline]
    pub fn iso_week(&self) -> u8 {
        self.inner.iso_week()
    }
}

impl Default for DateTime {
    fn default() -> Self {
        Self {
            inner: DateTimeImpl::UNIX_EPOCH,
        }
    }
}

const DATE_TIME_FORMAT: &[time::format_description::FormatItem<'static>] = time::macros::format_description!(
    "[year]-[month]-[day] [hour]:[minute]:[second] [offset_hour sign:mandatory][offset_minute]"
);

const DATE_TIME_FORMAT_SUBSEC: &[time::format_description::FormatItem<'static>] = time::macros::format_description!(
    "[year]-[month]-[day] [hour]:[minute]:[second].[subsecond] [offset_hour sign:mandatory][offset_minute]"
);

impl fmt::Display for DateTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let date_format = match self.inner.nanosecond() {
            0 => DATE_TIME_FORMAT,
            _ => DATE_TIME_FORMAT_SUBSEC,
        };

        write!(
            f,
            "{}",
            self.inner.format(date_format).map_err(|_e| fmt::Error)?
        )
    }
}

impl ops::Deref for DateTime {
    type Target = DateTimeImpl;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl ops::DerefMut for DateTime {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

mod friendly_date_time {
    use super::*;
    use serde::{self, Deserialize, Deserializer, Serializer};

    pub(crate) fn serialize<S>(date: &DateTimeImpl, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let date_format = match date.nanosecond() {
            0 => DATE_TIME_FORMAT,
            _ => DATE_TIME_FORMAT_SUBSEC,
        };

        let s = date
            .format(date_format)
            .map_err(serde::ser::Error::custom)?;
        serializer.serialize_str(&s)
    }

    pub(crate) fn deserialize<'de, D>(deserializer: D) -> Result<DateTimeImpl, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s: std::borrow::Cow<'_, str> = Deserialize::deserialize(deserializer)?;
        if let Ok(date) = DateTimeImpl::parse(&s, DATE_TIME_FORMAT_SUBSEC) {
            Ok(date)
        } else {
            DateTimeImpl::parse(&s, DATE_TIME_FORMAT).map_err(serde::de::Error::custom)
        }
    }
}

/// Parse a string representing the date and time.
///
/// Accepts any of the formats listed below and builds return an `Option`
/// containing a `DateTimeImpl`.
///
/// Supported formats:
///
/// * `default` - `YYYY-MM-DD HH:MM:SS`
/// * `day_month` - `DD Month YYYY HH:MM:SS`
/// * `day_mon` - `DD Mon YYYY HH:MM:SS`
/// * `mdy` -  `MM/DD/YYYY HH:MM:SS`
/// * `dow_mon` - `Dow Mon DD HH:MM:SS YYYY`
///
/// Offsets in one of the following forms, and are catenated with any of
/// the above formats.
///
/// * `+HHMM`
/// * `-HHMM`
///
/// Example:
///
/// * `dow_mon` format with an offset: "Tue Feb 16 10:00:00 2016 +0100"
fn parse_date_time(s: &str) -> Option<DateTimeImpl> {
    use regex::Regex;

    if s.is_empty() {
        return None
    }

    if let "now" | "today" = s.to_lowercase().trim() {
        let offset_in_sec = chrono::Local::now()
            .offset()
            .local_minus_utc();
        let offset = UtcOffset::from_whole_seconds(offset_in_sec).unwrap();

        let utc_now = DateTimeImpl::now_utc();
        return Some(utc_now.to_offset(offset));
    }

    if let Ok(unix_ts) = s.parse::<i64>() {
        if unix_ts >= 946702800 && // 2000-Jan-1
            unix_ts < 4102462800 { // 2100-Jan-1

            if let Ok(dt) = DateTimeImpl::from_unix_timestamp(unix_ts) {
                let tz = rb_date_parser::get_current_timezone();
                let local_time = tz.timestamp_opt(unix_ts, 0).earliest().unwrap();
                let offset_in_sec = local_time.fixed_offset().offset().local_minus_utc();

                let offset = UtcOffset::from_whole_seconds(offset_in_sec).unwrap();
                return Some(dt.to_offset(offset));
            }
        }
    }

    let offset_re = Regex::new(r"[+-][01][0-9]{3}$").unwrap();
    let offset = if offset_re.is_match(s) { "" } else { " +0000" };
    let s = s.to_owned() + offset;

    let dt = if let Ok(dt) = rb_date_parser::parse(s.as_str()) {
        dt
    } else {
        return None;
    };
    let year = dt.year();
    let mut month = dt.month();
    let mut day = dt.day();
    if month > 12 {
        std::mem::swap(&mut month, &mut day);
    }
    let date = time::Date::from_calendar_date(year, time::Month::try_from(month as u8).unwrap(), day as u8).unwrap();
    let hour = dt.hour();
    let min = dt.minute();
    let sec = dt.second();
    let nanos = dt.nanosecond();
    let time = time::Time::from_hms_nano(hour as u8, min as u8, sec as u8, nanos).unwrap();
    let offset = time::UtcOffset::from_whole_seconds(dt.offset().local_minus_utc()).unwrap();

    let res = Some(time::OffsetDateTime::new_in_offset(date, time, offset));
    res
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn parse_date_time_empty_is_bad() {
        let input = "";
        let actual = parse_date_time(input);
        assert!(actual.is_none());
    }

    #[test]
    fn parse_date_time_bad() {
        let input = "aaaaa";
        let actual = parse_date_time(input);
        assert!(actual.is_none());
    }

    #[test]
    fn parse_date_time_now() {
        let input = "now";
        let actual = parse_date_time(input);
        assert!(actual.is_some());
    }

    #[test]
    fn parse_date_time_today() {
        let input = "today";
        let actual = parse_date_time(input);
        assert!(actual.is_some());

        let input = "Today";
        let actual = parse_date_time(input);
        assert!(actual.is_some());
    }

    #[test]
    fn parse_date_time_serialized_format() {
        let input = "2016-02-16 10:00:00 +0100"; // default format with offset
        let actual = parse_date_time(input);
        assert!(actual.unwrap().unix_timestamp() == 1455613200);

        let input = "2016-02-16 10:00:00 +0000"; // default format UTC
        let actual = parse_date_time(input);
        assert!(actual.unwrap().unix_timestamp() == 1455616800);

        let input = "2016-02-16 10:00:00"; // default format no offset
        let actual = parse_date_time(input);
        assert!(actual.unwrap().unix_timestamp() == 1455616800);
    }

    #[test]
    fn parse_date_time_serialized_format_with_subseconds() {
        let input = "2016-02-16 10:00:00.123456789 +0100"; // default format with offset
        let actual = parse_date_time(input);
        assert!(actual.unwrap().unix_timestamp_nanos() == 1455613200123456789);

        let input = "2016-02-16 10:00:00.123456789 +0000"; // default format UTC
        let actual = parse_date_time(input);
        assert!(actual.unwrap().unix_timestamp_nanos() == 1455616800123456789);

        let input = "2016-02-16 10:00:00.123456789"; // default format no offset
        let actual = parse_date_time(input);
        assert!(actual.unwrap().unix_timestamp_nanos() == 1455616800123456789);
    }

    #[test]
    fn parse_date_time_day_month_format() {
        let input = "16 February 2016 10:00:00 +0100"; // day_month format with offset
        let actual = parse_date_time(input);
        assert!(actual.unwrap().unix_timestamp() == 1455613200);

        let input = "16 February 2016 10:00:00 +0000"; // day_month format UTC
        let actual = parse_date_time(input);
        assert!(actual.unwrap().unix_timestamp() == 1455616800);

        let input = "16 February 2016 10:00:00"; // day_month format no offset
        let actual = parse_date_time(input);
        assert!(actual.unwrap().unix_timestamp() == 1455616800);
    }

    #[test]
    fn parse_date_time_day_mon_format() {
        let input = "16 Feb 2016 10:00:00 +0100"; // day_mon format with offset
        let actual = parse_date_time(input);
        assert!(actual.unwrap().unix_timestamp() == 1455613200);

        let input = "16 Feb 2016 10:00:00 +0000"; // day_mon format UTC
        let actual = parse_date_time(input);
        assert!(actual.unwrap().unix_timestamp() == 1455616800);

        let input = "16 Feb 2016 10:00:00"; // day_mon format no offset
        let actual = parse_date_time(input);
        assert!(actual.unwrap().unix_timestamp() == 1455616800);
    }

    #[test]
    fn parse_date_time_mdy_format() {
        let input = "02/16/2016 10:00:00 +0100"; // mdy format with offset
        let actual = parse_date_time(input);
        assert!(actual.unwrap().unix_timestamp() == 1455613200);

        let input = "02/16/2016 10:00:00 +0000"; // mdy format UTC
        let actual = parse_date_time(input);
        assert!(actual.unwrap().unix_timestamp() == 1455616800);

        let input = "02/16/2016 10:00:00"; // mdy format no offset
        let actual = parse_date_time(input);
        assert!(actual.unwrap().unix_timestamp() == 1455616800);
    }

    #[test]
    fn parse_date_time_dow_mon_format() {
        let input = "Tue Feb 16 10:00:00 2016 +0100"; // dow_mon format with offset
        let actual = parse_date_time(input);
        assert!(actual.unwrap().unix_timestamp() == 1455613200);

        let input = "Tue Feb 16 10:00:00 2016 +0000"; // dow_mon format UTC
        let actual = parse_date_time(input);
        assert!(actual.unwrap().unix_timestamp() == 1455616800);

        let input = "Tue Feb 16 10:00:00 2016"; // dow_mon format no offset
        let actual = parse_date_time(input);
        assert!(actual.unwrap().unix_timestamp() == 1455616800);
    }

    #[test]
    fn parse_date_time_to_string() {
        let date = DateTime::now();
        let input = date.to_string();
        let actual = parse_date_time(&input);
        assert!(actual.is_some());
    }

    #[derive(serde::Serialize, serde::Deserialize)]
    struct TestSerde {
        date: DateTime,
    }

    #[test]
    fn serialize_deserialize_date_time() {
        let yml = "---\ndate: \"2021-05-02 21:00:00 +0100\"\n";
        let data: TestSerde = serde_yaml::from_str(yml).expect("could deserialize date");
        let ser = serde_yaml::to_string(&data).expect("could serialize date");
        assert_eq!(yml, ser);
    }

    #[test]
    fn serialize_deserialize_date_time_ms() {
        let yml = "---\ndate: \"2021-05-02 21:00:00.12 +0100\"\n";
        let data: TestSerde = serde_yaml::from_str(yml).expect("could deserialize date");
        let ser = serde_yaml::to_string(&data).expect("could serialize date");
        assert_eq!(yml, ser);
    }
}
