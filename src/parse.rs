use crate::datetime::{
    CalendarDate, DateTime, Day, Hour24, Minute, Month, Second, TimeOfDay, UtcOffset, Year,
    from_unix_timestamp, to_unix_timestamp,
};
use crate::utc_offset::utc_offset_seconds_at;
use std::str::FromStr;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Debug, PartialEq, Eq)]
pub enum ParseDurationError {
    InvalidInput,
    EmptyInput,
    InvalidNumber,
    InvalidUnit,
    Overflow,
}

impl ParseDurationError {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::InvalidInput => "Invalid input",
            Self::EmptyInput => "Empty input",
            Self::InvalidNumber => "Invalid number",
            Self::InvalidUnit => "Invalid unit",
            Self::Overflow => "Duration overflow",
        }
    }
}

impl std::fmt::Display for ParseDurationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SleepDuration {
    Finite(Duration),
    Infinite,
}

impl FromStr for SleepDuration {
    type Err = ParseDurationError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let input = input.trim();
        if input.is_empty() {
            return Err(ParseDurationError::EmptyInput);
        }

        if input.parse::<InfiniteDuration>().is_ok() {
            return Ok(Self::Infinite);
        }

        let finite = input.strip_prefix('+').unwrap_or(input);
        parse_duration(finite).map(Self::Finite)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct InfiniteDuration;

impl FromStr for InfiniteDuration {
    type Err = ParseDurationError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let number = match input.chars().next_back() {
            Some(unit) if matches!(unit, 's' | 'S' | 'm' | 'M' | 'h' | 'H' | 'd' | 'D') => {
                &input[..input.len() - unit.len_utf8()]
            }
            _ => input,
        };

        if number.eq_ignore_ascii_case("inf") || number.eq_ignore_ascii_case("infinity") {
            Ok(Self)
        } else {
            Err(ParseDurationError::InvalidNumber)
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum ParseDateTimeError {
    InvalidFormat,
    InvalidValue,
    PastTime,
    Overflow,
}

impl ParseDateTimeError {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::InvalidFormat => "Invalid datetime format",
            Self::InvalidValue => "Invalid date/time value",
            Self::PastTime => "Specified time is in the past",
            Self::Overflow => "Datetime is out of range",
        }
    }
}

impl std::fmt::Display for ParseDateTimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl FromStr for UtcOffset {
    type Err = ParseDateTimeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        let (sign, value) = if let Some(value) = s.strip_prefix('+') {
            (1, value)
        } else if let Some(value) = s.strip_prefix('-') {
            (-1, value)
        } else {
            return Err(ParseDateTimeError::InvalidFormat);
        };

        let (hours, minutes) = match value.split_once(':') {
            Some((hours, minutes)) => (parse_u8(hours)?, parse_u8(minutes)?),
            None if value.len() == 4 => {
                let (hours, minutes) = value.split_at(2);
                (parse_u8(hours)?, parse_u8(minutes)?)
            }
            None if value.len() == 2 => (parse_u8(value)?, 0),
            None => return Err(ParseDateTimeError::InvalidFormat),
        };

        let hours = Hour24::new(hours).ok_or(ParseDateTimeError::InvalidValue)?;
        let minutes = Minute::new(minutes).ok_or(ParseDateTimeError::InvalidValue)?;
        Ok(UtcOffset::from_seconds(
            sign * (i32::from(hours.0) * 3600 + i32::from(minutes.0) * 60),
        ))
    }
}

fn parse_u64(s: &str) -> Option<u64> {
    if s.is_empty() {
        return None;
    }
    let mut res = 0u64;
    for &b in s.as_bytes() {
        if b.is_ascii_digit() {
            res = res.checked_mul(10)?.checked_add((b - b'0') as u64)?;
        } else {
            return None;
        }
    }
    Some(res)
}

fn parse_u8(s: &str) -> Result<u8, ParseDateTimeError> {
    let value = parse_u64(s).ok_or(ParseDateTimeError::InvalidFormat)?;
    u8::try_from(value).map_err(|_| ParseDateTimeError::InvalidValue)
}

#[derive(Debug, Clone, Copy)]
enum DurationNumber {
    Integer(u64),
    Float(f64),
}

impl std::str::FromStr for DurationNumber {
    type Err = ParseDurationError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty()
            || s.starts_with('+')
            || s.starts_with('-')
            || !matches!(s.chars().next(), Some('0'..='9' | '.'))
        {
            return Err(ParseDurationError::InvalidInput);
        }

        if s.contains('.') || s.contains('e') || s.contains('E') {
            s.parse::<f64>()
                .map(Self::Float)
                .map_err(|_| ParseDurationError::InvalidNumber)
        } else {
            s.parse::<u64>()
                .map(Self::Integer)
                .map_err(|_| ParseDurationError::InvalidNumber)
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum DurationUnit {
    Seconds,
    Minutes,
    Hours,
    Days,
}

impl DurationUnit {
    fn from_suffix(suffix: char) -> Option<Self> {
        match suffix {
            's' => Some(Self::Seconds),
            'm' => Some(Self::Minutes),
            'h' => Some(Self::Hours),
            'd' => Some(Self::Days),
            _ => None,
        }
    }

    fn multiplier(self) -> u64 {
        match self {
            Self::Seconds => 1,
            Self::Minutes => 60,
            Self::Hours => 3600,
            Self::Days => 86400,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct DurationPart(Duration);

impl std::str::FromStr for DurationPart {
    type Err = ParseDurationError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.is_empty() {
            return Err(ParseDurationError::InvalidInput);
        }

        let (number, unit) = match s.char_indices().next_back() {
            Some((index, suffix)) => {
                if let Some(unit) = DurationUnit::from_suffix(suffix) {
                    (&s[..index], unit)
                } else {
                    match s.parse::<DurationNumber>() {
                        Ok(number) => return Self::from_number(number, DurationUnit::Seconds),
                        Err(error) => {
                            if !matches!(suffix, 'e' | 'E')
                                && s[..index].parse::<DurationNumber>().is_ok()
                            {
                                return Err(ParseDurationError::InvalidUnit);
                            }
                            return Err(error);
                        }
                    }
                }
            }
            None => return Err(ParseDurationError::InvalidInput),
        };

        let number = number.parse::<DurationNumber>()?;
        Self::from_number(number, unit)
    }
}

impl DurationPart {
    fn from_number(number: DurationNumber, unit: DurationUnit) -> Result<Self, ParseDurationError> {
        let multiplier = unit.multiplier();
        let duration = match number {
            DurationNumber::Integer(value) => {
                let seconds = value
                    .checked_mul(multiplier)
                    .ok_or(ParseDurationError::Overflow)?;
                Duration::from_secs(seconds)
            }
            DurationNumber::Float(value) => {
                let seconds = value * multiplier as f64;
                Duration::try_from_secs_f64(seconds).map_err(|_| ParseDurationError::Overflow)?
            }
        };

        Ok(Self(duration))
    }
}

fn parse_duration_chunk(chunk: &str) -> Result<Duration, ParseDurationError> {
    let mut total = Duration::ZERO;
    let mut start = 0;

    for (index, ch) in chunk.char_indices() {
        if DurationUnit::from_suffix(ch).is_none() {
            continue;
        }

        let end = index + ch.len_utf8();
        let part = chunk[start..end].parse::<DurationPart>()?;
        total = total
            .checked_add(part.0)
            .ok_or(ParseDurationError::Overflow)?;
        start = end;
    }

    if start < chunk.len() {
        let part = chunk[start..].parse::<DurationPart>()?;
        total = total
            .checked_add(part.0)
            .ok_or(ParseDurationError::Overflow)?;
    } else if start == 0 {
        return Err(ParseDurationError::InvalidInput);
    }

    Ok(total)
}

pub fn parse_duration(s: &str) -> Result<Duration, ParseDurationError> {
    let s = s.trim();
    if s.is_empty() {
        return Err(ParseDurationError::EmptyInput);
    }

    s.split_ascii_whitespace()
        .try_fold(Duration::ZERO, |total, chunk| {
            total
                .checked_add(parse_duration_chunk(chunk)?)
                .ok_or(ParseDurationError::Overflow)
        })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Meridiem {
    Am,
    Pm,
}

impl FromStr for Meridiem {
    type Err = ParseDateTimeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.eq_ignore_ascii_case("am") {
            Ok(Self::Am)
        } else if s.eq_ignore_ascii_case("pm") {
            Ok(Self::Pm)
        } else {
            Err(ParseDateTimeError::InvalidFormat)
        }
    }
}

impl Meridiem {
    fn to_24_hour(self, hour: u8) -> Result<Hour24, ParseDateTimeError> {
        if !(1..=12).contains(&hour) {
            return Err(ParseDateTimeError::InvalidValue);
        }

        let hour = match (self, hour) {
            (Self::Am, 12) => 0,
            (Self::Pm, 12) => 12,
            (Self::Pm, hour) => hour + 12,
            (Self::Am, hour) => hour,
        };

        Hour24::new(hour).ok_or(ParseDateTimeError::InvalidValue)
    }
}

impl FromStr for CalendarDate {
    type Err = ParseDateTimeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut parts = s.split('-');
        let year = parts.next().ok_or(ParseDateTimeError::InvalidFormat)?;
        let month = parts.next().ok_or(ParseDateTimeError::InvalidFormat)?;
        let day = parts.next().ok_or(ParseDateTimeError::InvalidFormat)?;
        if parts.next().is_some() || year.len() != 4 || month.len() != 2 || day.len() != 2 {
            return Err(ParseDateTimeError::InvalidFormat);
        }

        let year = parse_u64(year).ok_or(ParseDateTimeError::InvalidFormat)? as i32;
        let year = Year(year);
        let month = Month::new(parse_u8(month)?).ok_or(ParseDateTimeError::InvalidValue)?;
        let day = Day::new(parse_u8(day)?, year, month).ok_or(ParseDateTimeError::InvalidValue)?;

        Ok(Self { year, month, day })
    }
}

impl FromStr for TimeOfDay {
    type Err = ParseDateTimeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        let (time, meridiem) = split_meridiem(s);

        let mut parts = time.split(':');
        let hour = parse_u8(parts.next().ok_or(ParseDateTimeError::InvalidFormat)?)?;
        let minute = match parts.next() {
            Some(value) => parse_u8(value)?,
            None if meridiem.is_some() => 0,
            None => return Err(ParseDateTimeError::InvalidFormat),
        };
        let second = match parts.next() {
            Some(value) => parse_u8(value)?,
            None => 0,
        };
        if parts.next().is_some() {
            return Err(ParseDateTimeError::InvalidFormat);
        }

        let hour = match meridiem {
            Some(meridiem) => meridiem.to_24_hour(hour)?,
            None => Hour24::new(hour).ok_or(ParseDateTimeError::InvalidValue)?,
        };
        let minute = Minute::new(minute).ok_or(ParseDateTimeError::InvalidValue)?;
        let second = Second::new(second).ok_or(ParseDateTimeError::InvalidValue)?;

        Ok(Self {
            hour,
            minute,
            second,
        })
    }
}

fn split_meridiem(s: &str) -> (&str, Option<Meridiem>) {
    let Some(suffix_start) = s.len().checked_sub(2) else {
        return (s, None);
    };
    let Some((time, suffix)) = s.split_at_checked(suffix_start) else {
        return (s, None);
    };
    let Ok(meridiem) = suffix.parse::<Meridiem>() else {
        return (s, None);
    };

    (time.trim(), Some(meridiem))
}

fn strip_ascii_suffix_ignore_case<'a>(s: &'a str, suffix: &str) -> Option<&'a str> {
    let suffix_start = s.len().checked_sub(suffix.len())?;
    let (prefix, actual_suffix) = s.split_at_checked(suffix_start)?;
    actual_suffix.eq_ignore_ascii_case(suffix).then_some(prefix)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TimeSpec {
    time: TimeOfDay,
    offset: Option<UtcOffset>,
}

impl FromStr for TimeSpec {
    type Err = ParseDateTimeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        let (time, offset) = if let Some(time) = s.strip_suffix('Z') {
            (time, Some(UtcOffset::from_seconds(0)))
        } else if let Some(time) = strip_ascii_suffix_ignore_case(s, "utc") {
            (time.trim(), Some(UtcOffset::from_seconds(0)))
        } else if let Some(pos) = s.find(['+', '-']) {
            let (time, offset) = s.split_at(pos);
            (time.trim(), Some(offset.parse::<UtcOffset>()?))
        } else {
            (s, None)
        };

        Ok(Self {
            time: time.parse::<TimeOfDay>()?,
            offset,
        })
    }
}

pub fn parse_datetime(s: &str, now: SystemTime) -> Result<SystemTime, ParseDateTimeError> {
    parse_datetime_with(s, now, |dt| {
        utc_offset_seconds_at(dt).ok_or(ParseDateTimeError::InvalidValue)
    })
}

pub fn parse_datetime_with<F>(
    s: &str,
    now: SystemTime,
    mut utc_offset_at: F,
) -> Result<SystemTime, ParseDateTimeError>
where
    F: FnMut(DateTime) -> Result<i32, ParseDateTimeError>,
{
    let s = s.trim();
    if s.is_empty() {
        return Err(ParseDateTimeError::InvalidFormat);
    }

    if let Some(ts_str) = s.strip_prefix('@') {
        return parse_unix_timestamp(now, ts_str);
    }

    let now_ts_utc = now
        .duration_since(UNIX_EPOCH)
        .map_err(|_| ParseDateTimeError::InvalidValue)?
        .as_secs();

    if s == "tomorrow" {
        return target_tomorrow_time(now_ts_utc, "00:00", &mut utc_offset_at);
    }

    if let Some(time_part) = s.strip_prefix("tomorrow ") {
        return target_tomorrow_time(now_ts_utc, time_part, &mut utc_offset_at);
    }

    let (date_part, time_part) = match s.split_once(' ') {
        Some((date, time)) => (date, (!time.is_empty()).then_some(time)),
        None => (s, None),
    };
    match date_part.parse::<CalendarDate>() {
        Ok(date) => {
            let time_spec = match time_part {
                Some(time) => time.parse::<TimeSpec>()?,
                None => TimeSpec {
                    time: TimeOfDay::MIDNIGHT,
                    offset: None,
                },
            };

            let target_dt = DateTime {
                year: date.year,
                month: date.month,
                day: date.day,
                hour: time_spec.time.hour,
                minute: time_spec.time.minute,
                second: time_spec.time.second,
            };

            let ts_utc = target_to_utc(target_dt, time_spec.offset, &mut utc_offset_at)?;
            let target = UNIX_EPOCH
                .checked_add(Duration::from_secs(ts_utc))
                .ok_or(ParseDateTimeError::Overflow)?;
            if target <= now {
                return Err(ParseDateTimeError::PastTime);
            }
            return Ok(target);
        }
        Err(ParseDateTimeError::InvalidValue) => return Err(ParseDateTimeError::InvalidValue),
        Err(_) => {}
    }

    if let Ok(time_spec) = s.parse::<TimeSpec>() {
        let current_dt = match time_spec.offset {
            Some(offset) => datetime_at_offset(now_ts_utc, offset)?,
            None => local_datetime_at(now_ts_utc, &mut utc_offset_at)?,
        };
        let mut target_dt = DateTime {
            hour: time_spec.time.hour,
            minute: time_spec.time.minute,
            second: time_spec.time.second,
            ..current_dt
        };

        let mut ts_utc = target_to_utc(target_dt, time_spec.offset, &mut utc_offset_at)?;
        let mut target = UNIX_EPOCH
            .checked_add(Duration::from_secs(ts_utc))
            .ok_or(ParseDateTimeError::Overflow)?;
        if target <= now {
            target_dt = next_calendar_day(target_dt)?;
            ts_utc = target_to_utc(target_dt, time_spec.offset, &mut utc_offset_at)?;
            target = UNIX_EPOCH
                .checked_add(Duration::from_secs(ts_utc))
                .ok_or(ParseDateTimeError::Overflow)?;
        }
        return Ok(target);
    }

    Err(ParseDateTimeError::InvalidFormat)
}

fn target_tomorrow_time<F>(
    now_ts_utc: u64,
    time_part: &str,
    utc_offset_at: &mut F,
) -> Result<SystemTime, ParseDateTimeError>
where
    F: FnMut(DateTime) -> Result<i32, ParseDateTimeError>,
{
    let time_spec = time_part.parse::<TimeSpec>()?;
    let current_dt = match time_spec.offset {
        Some(offset) => datetime_at_offset(now_ts_utc, offset)?,
        None => local_datetime_at(now_ts_utc, utc_offset_at)?,
    };
    let tomorrow = next_calendar_day(current_dt)?;
    let target_dt = DateTime {
        hour: time_spec.time.hour,
        minute: time_spec.time.minute,
        second: time_spec.time.second,
        ..tomorrow
    };
    let target_as_utc = target_to_utc(target_dt, time_spec.offset, utc_offset_at)?;
    UNIX_EPOCH
        .checked_add(Duration::from_secs(target_as_utc))
        .ok_or(ParseDateTimeError::Overflow)
}

fn local_datetime_at<F>(
    now_ts_utc: u64,
    utc_offset_at: &mut F,
) -> Result<DateTime, ParseDateTimeError>
where
    F: FnMut(DateTime) -> Result<i32, ParseDateTimeError>,
{
    let utc_dt = from_unix_timestamp(now_ts_utc);
    let initial_offset = utc_offset_at(utc_dt)?;
    let initial_local = datetime_at_offset(now_ts_utc, UtcOffset::from_seconds(initial_offset))?;
    let resolved_offset = utc_offset_at(initial_local)?;

    if resolved_offset == initial_offset {
        Ok(initial_local)
    } else {
        datetime_at_offset(now_ts_utc, UtcOffset::from_seconds(resolved_offset))
    }
}

fn datetime_at_offset(ts_utc: u64, offset: UtcOffset) -> Result<DateTime, ParseDateTimeError> {
    let offset = offset.seconds();
    let shifted = if offset >= 0 {
        ts_utc.checked_add(offset as u64)
    } else {
        ts_utc.checked_sub(offset.unsigned_abs() as u64)
    }
    .ok_or(ParseDateTimeError::InvalidValue)?;

    Ok(from_unix_timestamp(shifted))
}

fn next_calendar_day(dt: DateTime) -> Result<DateTime, ParseDateTimeError> {
    let next = to_unix_timestamp(dt)
        .checked_add(86400)
        .ok_or(ParseDateTimeError::InvalidValue)?;
    Ok(from_unix_timestamp(next))
}

fn parse_unix_timestamp(now: SystemTime, ts_str: &str) -> Result<SystemTime, ParseDateTimeError> {
    let ts = parse_u64(ts_str).ok_or(ParseDateTimeError::InvalidFormat)?;
    let target = UNIX_EPOCH
        .checked_add(Duration::from_secs(ts))
        .ok_or(ParseDateTimeError::Overflow)?;
    if target <= now {
        return Err(ParseDateTimeError::PastTime);
    }
    Ok(target)
}

fn target_to_utc<F>(
    dt: DateTime,
    target_offset: Option<UtcOffset>,
    utc_offset_at: &mut F,
) -> Result<u64, ParseDateTimeError>
where
    F: FnMut(DateTime) -> Result<i32, ParseDateTimeError>,
{
    let ts_target = to_unix_timestamp(dt);
    let used_offset = match target_offset {
        Some(offset) => offset.seconds(),
        None => utc_offset_at(dt)?,
    };

    if used_offset >= 0 {
        ts_target
            .checked_sub(used_offset as u64)
            .ok_or(ParseDateTimeError::InvalidValue)
    } else {
        ts_target
            .checked_add(used_offset.unsigned_abs() as u64)
            .ok_or(ParseDateTimeError::InvalidValue)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seconds_only() {
        assert_eq!(parse_duration("90").unwrap(), Duration::from_secs(90));
    }

    #[test]
    fn test_single_units() {
        assert_eq!(parse_duration("5s").unwrap(), Duration::from_secs(5));
        assert_eq!(parse_duration("5m").unwrap(), Duration::from_secs(300));
        assert_eq!(parse_duration("1h").unwrap(), Duration::from_secs(3600));
        assert_eq!(parse_duration("1d").unwrap(), Duration::from_secs(86400));
    }

    #[test]
    fn test_gnu_numeric_formats() {
        assert_eq!(parse_duration("0.5s").unwrap(), Duration::from_millis(500));
        assert_eq!(parse_duration("1e-3").unwrap(), Duration::from_millis(1));
        assert_eq!(parse_duration("1.5m").unwrap(), Duration::from_secs(90));
    }

    #[test]
    fn test_additional_gnu_numeric_forms() {
        assert_eq!(
            "+0.5s".parse::<SleepDuration>().unwrap(),
            SleepDuration::Finite(Duration::from_millis(500))
        );
        assert_eq!(
            "+1e-3".parse::<SleepDuration>().unwrap(),
            SleepDuration::Finite(Duration::from_millis(1))
        );
        assert_eq!(
            "infinity".parse::<SleepDuration>().unwrap(),
            SleepDuration::Infinite
        );
        assert_eq!(
            "infinitys".parse::<SleepDuration>().unwrap(),
            SleepDuration::Infinite
        );
        assert_eq!(
            "INFD".parse::<SleepDuration>().unwrap(),
            SleepDuration::Infinite
        );
    }

    #[test]
    fn test_combined_units() {
        assert_eq!(
            parse_duration("1h30m").unwrap(),
            Duration::from_secs(3600 + 1800)
        );
        assert_eq!(
            parse_duration("1d2h3m4s").unwrap(),
            Duration::from_secs(86400 + 7200 + 180 + 4)
        );
    }

    #[test]
    fn test_whitespace() {
        assert_eq!(
            parse_duration(" 1h 30m ").unwrap(),
            Duration::from_secs(5400)
        );
    }

    #[test]
    fn test_unit_suffixes_are_case_sensitive() {
        assert!(parse_duration("1H30M").is_err());
        assert!(parse_duration("42D").is_err());
    }

    #[test]
    fn test_empty() {
        assert_eq!(parse_duration(""), Err(ParseDurationError::EmptyInput));
        assert_eq!(parse_duration("   "), Err(ParseDurationError::EmptyInput));
    }

    #[test]
    fn test_invalid() {
        assert!(parse_duration("abc").err().is_some());
        assert!(parse_duration("1x").err().is_some());
    }

    #[test]
    fn test_overflow() {
        assert_eq!(
            parse_duration("1000000000000000000000d").err(),
            Some(ParseDurationError::InvalidNumber)
        );
        assert_eq!(
            parse_duration("9999999999999999999d").err(),
            Some(ParseDurationError::Overflow)
        );
    }

    #[test]
    fn test_time_of_day_parsing() {
        assert_eq!(
            "06:30:15".parse::<TimeOfDay>().unwrap(),
            TimeOfDay {
                hour: Hour24(6),
                minute: Minute(30),
                second: Second(15),
            }
        );
        assert_eq!(
            "6pm".parse::<TimeOfDay>().unwrap(),
            TimeOfDay {
                hour: Hour24(18),
                minute: Minute(0),
                second: Second(0),
            }
        );
        assert_eq!("12am".parse::<TimeOfDay>().unwrap(), TimeOfDay::MIDNIGHT);
        assert_eq!(
            "11:30PM".parse::<TimeOfDay>().unwrap(),
            TimeOfDay {
                hour: Hour24(23),
                minute: Minute(30),
                second: Second(0),
            }
        );
    }

    #[test]
    fn test_time_of_day_rejects_invalid_values() {
        assert_eq!(
            "0pm".parse::<TimeOfDay>(),
            Err(ParseDateTimeError::InvalidValue)
        );
        assert_eq!(
            "06".parse::<TimeOfDay>(),
            Err(ParseDateTimeError::InvalidFormat)
        );
        assert_eq!(
            "24:00".parse::<TimeOfDay>(),
            Err(ParseDateTimeError::InvalidValue)
        );
    }

    #[test]
    fn local_datetime_uses_offset_at_target_date() {
        let target = parse_datetime_with("2026-11-01 10:00", UNIX_EPOCH, |dt| {
            if dt.month.0 >= 11 { Ok(3600) } else { Ok(7200) }
        })
        .unwrap();

        assert_eq!(target, UNIX_EPOCH + Duration::from_secs(1793523600));
    }

    #[test]
    fn tomorrow_uses_offset_at_target_date() {
        let now = UNIX_EPOCH + Duration::from_secs(1793482200); // 2026-10-31 21:30 UTC
        let target = parse_datetime_with("tomorrow 10:00", now, |dt| {
            if dt.month.0 >= 11 { Ok(3600) } else { Ok(7200) }
        })
        .unwrap();

        assert_eq!(target, UNIX_EPOCH + Duration::from_secs(1793523600));
    }

    #[test]
    fn time_only_uses_offset_at_target_date() {
        let now = UNIX_EPOCH + Duration::from_secs(1793482200); // 2026-10-31 21:30 UTC
        let target = parse_datetime_with("10:00", now, |dt| {
            if dt.month.0 >= 11 { Ok(3600) } else { Ok(7200) }
        })
        .unwrap();

        assert_eq!(target, UNIX_EPOCH + Duration::from_secs(1793523600));
    }

    #[test]
    fn test_calendar_date() {
        assert_eq!(
            "2026-04-23".parse::<CalendarDate>().unwrap(),
            CalendarDate {
                year: Year(2026),
                month: Month(4),
                day: Day(23),
            }
        );
    }

    #[test]
    fn test_calendar_date_rejects_invalid_values() {
        assert_eq!(
            "2026-02-30".parse::<CalendarDate>(),
            Err(ParseDateTimeError::InvalidValue)
        );
        assert_eq!(
            "2026-13-01".parse::<CalendarDate>(),
            Err(ParseDateTimeError::InvalidValue)
        );
    }

    #[test]
    fn test_calendar_date_rejects_invalid_format() {
        assert_eq!(
            "2026/04/23".parse::<CalendarDate>(),
            Err(ParseDateTimeError::InvalidFormat)
        );
        assert_eq!(
            "2026-4-23".parse::<CalendarDate>(),
            Err(ParseDateTimeError::InvalidFormat)
        );
        assert_eq!(
            "2026-04-23X".parse::<CalendarDate>(),
            Err(ParseDateTimeError::InvalidFormat)
        );
    }

    #[test]
    fn test_parse_datetime_timestamp() {
        let now = UNIX_EPOCH + Duration::from_secs(1000);
        let target = parse_datetime("@2000", now).unwrap();
        assert_eq!(target.duration_since(UNIX_EPOCH).unwrap().as_secs(), 2000);
    }

    #[test]
    fn test_parse_datetime_timestamp_overflow_returns_error() {
        let result =
            std::panic::catch_unwind(|| parse_datetime("@18446744073709551615", UNIX_EPOCH));

        assert!(matches!(result, Ok(Err(ParseDateTimeError::Overflow))));
    }

    #[test]
    fn test_parse_datetime_rejects_invalid_full_date_separators() {
        assert_eq!(
            parse_datetime_with("2099-01-01X", UNIX_EPOCH, |_| Ok(0)),
            Err(ParseDateTimeError::InvalidFormat)
        );
        assert_eq!(
            parse_datetime_with("2099-01-01X12:00", UNIX_EPOCH, |_| Ok(0)),
            Err(ParseDateTimeError::InvalidFormat)
        );
    }

    #[test]
    fn test_parse_datetime_rejects_multibyte_suffix() {
        assert_eq!(
            parse_datetime_with("2099-01-01é", UNIX_EPOCH, |_| Ok(0)),
            Err(ParseDateTimeError::InvalidFormat)
        );
    }

    #[test]
    fn test_parse_datetime_utc_explicit() {
        let now = UNIX_EPOCH + Duration::from_secs(0);
        let target_z = parse_datetime("2026-04-23 08:00Z", now).unwrap();
        let expected = to_unix_timestamp(DateTime {
            year: Year(2026),
            month: Month(4),
            day: Day(23),
            hour: Hour24(8),
            minute: Minute(0),
            second: Second(0),
        });
        assert_eq!(
            target_z.duration_since(UNIX_EPOCH).unwrap().as_secs(),
            expected
        );

        let target_utc = parse_datetime("2026-04-23 08:00 UTC", now).unwrap();
        assert_eq!(
            target_utc.duration_since(UNIX_EPOCH).unwrap().as_secs(),
            expected
        );
    }

    #[test]
    fn test_parse_datetime_rejects_wrapped_time_fields() {
        assert!(parse_datetime("256:30", UNIX_EPOCH).is_err());
        assert!(parse_datetime("12:256", UNIX_EPOCH).is_err());
        assert!(parse_datetime("12:00:256", UNIX_EPOCH).is_err());
    }

    #[test]
    fn test_parse_utc_offset() {
        assert_eq!(
            "+02:30".parse::<UtcOffset>().unwrap(),
            UtcOffset::from_seconds(2 * 3600 + 30 * 60)
        );
        assert_eq!(
            "-0530".parse::<UtcOffset>().unwrap(),
            UtcOffset::from_seconds(-(5 * 3600 + 30 * 60))
        );
        assert_eq!(
            "+02".parse::<UtcOffset>().unwrap(),
            UtcOffset::from_seconds(2 * 3600)
        );
    }

    #[test]
    fn test_parse_utc_offset_rejects_invalid_values() {
        assert!("02:00".parse::<UtcOffset>().is_err());
        assert!("+24:00".parse::<UtcOffset>().is_err());
        assert!("+02:60".parse::<UtcOffset>().is_err());
        assert!("+020".parse::<UtcOffset>().is_err());
    }

    #[test]
    fn test_time_spec_utc() {
        let spec = "23:59Z".parse::<TimeSpec>().unwrap();
        assert_eq!(spec.offset, Some(UtcOffset::from_seconds(0)));
        assert_eq!(spec.time.hour, Hour24(23));

        let spec = "12:00 UTC".parse::<TimeSpec>().unwrap();
        assert_eq!(spec.offset, Some(UtcOffset::from_seconds(0)));
        assert_eq!(spec.time.hour, Hour24(12));

        let spec = "08:00".parse::<TimeSpec>().unwrap();
        assert_eq!(spec.offset, None);
    }

    #[test]
    fn test_time_spec_offset() {
        assert_eq!(
            "12:00+02:00".parse::<TimeSpec>().unwrap().offset,
            Some(UtcOffset::from_seconds(7200))
        );
        assert_eq!(
            "12:00-05:00".parse::<TimeSpec>().unwrap().offset,
            Some(UtcOffset::from_seconds(-18000))
        );
        assert_eq!(
            "12:00+0530".parse::<TimeSpec>().unwrap().offset,
            Some(UtcOffset::from_seconds(19800))
        );
    }

    #[test]
    fn test_parse_datetime_rejects_invalid_timezone_offsets() {
        assert!(parse_datetime("2099-01-01 12:00+24:00", UNIX_EPOCH).is_err());
        assert!(parse_datetime("2099-01-01 12:00+23:60", UNIX_EPOCH).is_err());
        assert!(parse_datetime("2099-01-01 12:00+99:99", UNIX_EPOCH).is_err());
        assert!(parse_datetime("2099-01-01 12:00+99999999999999999999:00", UNIX_EPOCH).is_err());
    }

    #[test]
    fn test_duration_parsing_mixed_units() {
        assert_eq!(
            parse_duration("1h30m10s").unwrap(),
            Duration::from_secs(3600 + 1800 + 10)
        );
        assert_eq!(
            parse_duration("1d2h3m4s").unwrap(),
            Duration::from_secs(86400 + 7200 + 180 + 4)
        );
    }

    #[test]
    fn test_duration_parsing_with_standalone_seconds() {
        assert_eq!(parse_duration("500").unwrap(), Duration::from_secs(500));
        assert_eq!(parse_duration("1m 1").unwrap(), Duration::from_secs(61));
        assert_eq!(parse_duration("2h 10").unwrap(), Duration::from_secs(7210));
    }

    #[test]
    fn test_time_of_day_am_midnight() {
        assert_eq!("12:00am".parse::<TimeOfDay>().unwrap(), TimeOfDay::MIDNIGHT);
        assert_eq!(
            "12:30am".parse::<TimeOfDay>().unwrap(),
            TimeOfDay {
                hour: Hour24(0),
                minute: Minute(30),
                second: Second(0),
            }
        );
    }

    #[test]
    fn test_time_of_day_am_morning() {
        assert_eq!("1:00am".parse::<TimeOfDay>().unwrap().hour, Hour24(1));
        assert_eq!("11:59am".parse::<TimeOfDay>().unwrap().hour, Hour24(11));
    }

    #[test]
    fn test_time_of_day_pm() {
        let noon = "12:00pm".parse::<TimeOfDay>().unwrap();
        assert_eq!(noon.hour, Hour24(12));
        assert_eq!(noon.minute, Minute(0));

        let afternoon = "12:30pm".parse::<TimeOfDay>().unwrap();
        assert_eq!(afternoon.hour, Hour24(12));
        assert_eq!(afternoon.minute, Minute(30));

        assert_eq!("1:00pm".parse::<TimeOfDay>().unwrap().hour, Hour24(13));
        assert_eq!("11:59pm".parse::<TimeOfDay>().unwrap().hour, Hour24(23));
    }

    #[test]
    fn test_time_of_day_am_pm_case_insensitivity() {
        assert_eq!("1:00AM".parse::<TimeOfDay>().unwrap().hour, Hour24(1));
        assert_eq!("1:00Pm".parse::<TimeOfDay>().unwrap().hour, Hour24(13));
    }

    #[test]
    fn test_time_of_day_am_pm_with_seconds() {
        let time = "1:00:05pm".parse::<TimeOfDay>().unwrap();
        assert_eq!(time.hour, Hour24(13));
        assert_eq!(time.second, Second(5));
    }

    #[test]
    fn test_time_spec_am_pm_with_offset() {
        assert_eq!(
            "1:00pm+02:00".parse::<TimeSpec>().unwrap().offset,
            Some(UtcOffset::from_seconds(7200))
        );
    }

    #[test]
    fn test_time_of_day_am_pm_hour_only() {
        let evening = "6pm".parse::<TimeOfDay>().unwrap();
        assert_eq!(evening.hour, Hour24(18));
        assert_eq!(evening.minute, Minute(0));

        assert_eq!("12am".parse::<TimeOfDay>().unwrap(), TimeOfDay::MIDNIGHT);

        let noon = "12PM".parse::<TimeOfDay>().unwrap();
        assert_eq!(noon.hour, Hour24(12));
        assert_eq!(noon.minute, Minute(0));
    }

    #[test]
    fn test_time_of_day_am_pm_invalid_hours() {
        assert!("0:00am".parse::<TimeOfDay>().is_err());
        assert!("13:00am".parse::<TimeOfDay>().is_err());
        assert!("0:00pm".parse::<TimeOfDay>().is_err());
        assert!("13:00pm".parse::<TimeOfDay>().is_err());
    }

    #[test]
    fn test_duration_parsing_errors() {
        assert_eq!(parse_duration("abc"), Err(ParseDurationError::InvalidInput));
        assert_eq!(
            parse_duration("-10s"),
            Err(ParseDurationError::InvalidInput)
        );
        assert_eq!(parse_duration("10x"), Err(ParseDurationError::InvalidUnit));
    }
}
