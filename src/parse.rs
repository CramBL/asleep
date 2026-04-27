use crate::datetime::{
    DateTime, Day, Hour24, Minute, Month, Second, Year, from_unix_timestamp, to_unix_timestamp,
};
use crate::duration::{Days, Hours, Minutes, Seconds};
use crate::utc_offset::utc_offset_seconds;
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

#[derive(Debug, PartialEq, Eq)]
pub enum ParseDateTimeError {
    InvalidFormat,
    InvalidValue,
    PastTime,
}

impl ParseDateTimeError {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::InvalidFormat => "Invalid datetime format",
            Self::InvalidValue => "Invalid date/time value",
            Self::PastTime => "Specified time is in the past",
        }
    }
}

impl std::fmt::Display for ParseDateTimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
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

pub fn parse_duration(s: &str) -> Result<Duration, ParseDurationError> {
    let s = s.trim();
    if s.is_empty() {
        return Err(ParseDurationError::EmptyInput);
    }

    let mut total_seconds = Seconds(0);
    let mut i = 0;
    let bytes = s.as_bytes();

    while i < bytes.len() {
        let c = bytes[i];
        if c.is_ascii_digit() {
            let start = i;
            while i < bytes.len() && bytes[i].is_ascii_digit() {
                i += 1;
            }
            let num = parse_u64(&s[start..i]).ok_or(ParseDurationError::InvalidNumber)?;

            if i < bytes.len() {
                let unit = bytes[i].to_ascii_lowercase();
                let multiplier: Seconds = match unit {
                    b's' => Seconds(1),
                    b'm' => Minutes(1).into(),
                    b'h' => Hours(1).into(),
                    b'd' => Days(1).into(),
                    _ => return Err(ParseDurationError::InvalidUnit),
                };
                i += 1;
                let added_seconds = num
                    .checked_mul(multiplier.0)
                    .map(Seconds)
                    .ok_or(ParseDurationError::Overflow)?;
                total_seconds = total_seconds
                    .checked_add(added_seconds)
                    .ok_or(ParseDurationError::Overflow)?;
            } else {
                total_seconds = total_seconds
                    .checked_add(Seconds(num))
                    .ok_or(ParseDurationError::Overflow)?;
            }
        } else if c.is_ascii_whitespace() {
            i += 1;
        } else {
            return Err(ParseDurationError::InvalidInput);
        }
    }

    Ok(Duration::from_secs(total_seconds.0))
}

pub fn parse_datetime(s: &str, now: SystemTime) -> Result<SystemTime, ParseDateTimeError> {
    let s = s.trim();
    if s.is_empty() {
        return Err(ParseDateTimeError::InvalidFormat);
    }

    if let Some(ts_str) = s.strip_prefix('@') {
        return parse_unix_timestamp(now, ts_str);
    }

    let local_offset = utc_offset_seconds() as i64;
    let now_ts_utc = now
        .duration_since(UNIX_EPOCH)
        .map_err(|_| ParseDateTimeError::InvalidValue)?
        .as_secs();

    if s == "tomorrow" {
        return Ok(tomorrow_time(local_offset, now_ts_utc));
    }

    if let Some(time_part) = s.strip_prefix("tomorrow ") {
        return target_tomorrow_time(local_offset, now_ts_utc, time_part);
    }

    if s.len() >= 10 && s.as_bytes()[4] == b'-' && s.as_bytes()[7] == b'-' {
        let year = parse_u64(&s[0..4]).ok_or(ParseDateTimeError::InvalidFormat)? as i32;
        let month = parse_u64(&s[5..7]).ok_or(ParseDateTimeError::InvalidFormat)? as u8;
        let day = parse_u64(&s[8..10]).ok_or(ParseDateTimeError::InvalidFormat)? as u8;

        let (h, min, sec, target_offset) = if s.len() > 11 {
            parse_time(&s[11..])?
        } else {
            (Hour24(0), Minute(0), Second(0), None)
        };

        let y = Year(year);
        let m = Month::new(month).ok_or(ParseDateTimeError::InvalidValue)?;
        let d = Day::new(day, y, m).ok_or(ParseDateTimeError::InvalidValue)?;

        let ts_utc = target_to_utc(
            DateTime {
                year: y,
                month: m,
                day: d,
                hour: h,
                minute: min,
                second: sec,
            },
            target_offset,
            local_offset,
        );
        let target = UNIX_EPOCH + Duration::from_secs(ts_utc);
        if target <= now {
            return Err(ParseDateTimeError::PastTime);
        }
        return Ok(target);
    }

    if let Ok((h, min, sec, target_offset)) = parse_time(s) {
        let used_offset = target_offset.unwrap_or(local_offset as i32) as i64;
        let now_ts_target = (now_ts_utc as i64 + used_offset) as u64;
        let dt = from_unix_timestamp(now_ts_target);

        let ts_utc = target_to_utc(
            DateTime {
                hour: h,
                minute: min,
                second: sec,
                ..dt
            },
            target_offset,
            local_offset,
        );
        let mut target = UNIX_EPOCH + Duration::from_secs(ts_utc);
        if target <= now {
            target += Duration::from_secs(86400);
        }
        return Ok(target);
    }

    Err(ParseDateTimeError::InvalidFormat)
}

fn target_tomorrow_time(
    local_offset: i64,
    now_ts_utc: u64,
    time_part: &str,
) -> Result<SystemTime, ParseDateTimeError> {
    let (h, min, sec, target_offset) = parse_time(time_part)?;
    let used_offset = target_offset.unwrap_or(local_offset as i32) as i64;
    let now_ts_target = (now_ts_utc as i64 + used_offset) as u64;
    let target_ts_target = now_ts_target + 86400;
    let dt = from_unix_timestamp(target_ts_target);
    let target_as_utc = target_to_utc(
        DateTime {
            hour: h,
            minute: min,
            second: sec,
            ..dt
        },
        target_offset,
        local_offset,
    );
    let target_time = UNIX_EPOCH + Duration::from_secs(target_as_utc);
    Ok(target_time)
}

fn tomorrow_time(local_offset: i64, now_ts_utc: u64) -> SystemTime {
    let now_ts_local = (now_ts_utc as i64 + local_offset) as u64;
    let target_ts_local = now_ts_local + 86400;
    let dt = from_unix_timestamp(target_ts_local);
    let target_as_utc = target_to_utc(
        DateTime {
            hour: Hour24(0),
            minute: Minute(0),
            second: Second(0),
            ..dt
        },
        None,
        local_offset,
    );
    UNIX_EPOCH + Duration::from_secs(target_as_utc)
}

fn parse_unix_timestamp(now: SystemTime, ts_str: &str) -> Result<SystemTime, ParseDateTimeError> {
    let ts = parse_u64(ts_str).ok_or(ParseDateTimeError::InvalidFormat)?;
    let target = UNIX_EPOCH + Duration::from_secs(ts);
    if target <= now {
        return Err(ParseDateTimeError::PastTime);
    }
    Ok(target)
}

fn target_to_utc(dt: DateTime, target_offset: Option<i32>, local_offset: i64) -> u64 {
    let ts_target = to_unix_timestamp(dt);
    let used_offset = target_offset.unwrap_or(local_offset as i32) as i64;
    (ts_target as i64 - used_offset) as u64
}

fn parse_time(s: &str) -> Result<(Hour24, Minute, Second, Option<i32>), ParseDateTimeError> {
    let s = s.trim();

    let (time_part_raw, offset) = if let Some(stripped) = s.strip_suffix('Z') {
        (stripped, Some(0))
    } else if s.to_ascii_lowercase().ends_with("utc") {
        let len = s.len() - 3;
        (s[..len].trim(), Some(0))
    } else if let Some(pos) = s.find(['+', '-']) {
        let (t, o_str) = s.split_at(pos);
        let offset = parse_offset(o_str).ok_or(ParseDateTimeError::InvalidFormat)?;
        (t.trim(), Some(offset))
    } else {
        (s, None)
    };

    let (time_part, am_pm) =
        if let Some(_stripped) = time_part_raw.to_ascii_lowercase().strip_suffix("am") {
            (time_part_raw[..time_part_raw.len() - 2].trim(), Some(false))
        } else if let Some(_stripped) = time_part_raw.to_ascii_lowercase().strip_suffix("pm") {
            (time_part_raw[..time_part_raw.len() - 2].trim(), Some(true))
        } else {
            (time_part_raw, None)
        };

    let mut parts = time_part.split(':');
    let h_str = parts.next().ok_or(ParseDateTimeError::InvalidFormat)?;
    let m_str = parts.next();
    let s_str = parts.next();
    if parts.next().is_some() {
        return Err(ParseDateTimeError::InvalidFormat);
    }

    let h_raw = parse_u64(h_str).ok_or(ParseDateTimeError::InvalidFormat)? as u8;
    let m = if let Some(m_val) = m_str {
        parse_u64(m_val).ok_or(ParseDateTimeError::InvalidFormat)? as u8
    } else {
        // Minutes are optional ONLY if am/pm is provided
        if am_pm.is_none() {
            return Err(ParseDateTimeError::InvalidFormat);
        }
        0
    };
    let s = if let Some(s_val) = s_str {
        parse_u64(s_val).ok_or(ParseDateTimeError::InvalidFormat)? as u8
    } else {
        0
    };

    let h = match am_pm {
        Some(is_pm) => {
            if h_raw == 0 || h_raw > 12 {
                return Err(ParseDateTimeError::InvalidValue);
            }
            if is_pm {
                if h_raw == 12 { 12 } else { h_raw + 12 }
            } else if h_raw == 12 {
                0
            } else {
                h_raw
            }
        }
        None => h_raw,
    };

    let hour = Hour24::new(h).ok_or(ParseDateTimeError::InvalidValue)?;
    let min = Minute::new(m).ok_or(ParseDateTimeError::InvalidValue)?;
    let sec = Second::new(s).ok_or(ParseDateTimeError::InvalidValue)?;

    Ok((hour, min, sec, offset))
}

fn parse_offset(s: &str) -> Option<i32> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }
    let sign = match s.as_bytes()[0] {
        b'+' => 1,
        b'-' => -1,
        _ => return None,
    };
    let s = &s[1..];
    let (h, m) = if let Some(pos) = s.find(':') {
        let h = parse_u64(&s[..pos])? as i32;
        let m = parse_u64(&s[pos + 1..])? as i32;
        (h, m)
    } else if s.len() == 4 {
        let h = parse_u64(&s[..2])? as i32;
        let m = parse_u64(&s[2..])? as i32;
        (h, m)
    } else if s.len() == 2 {
        let h = parse_u64(s)? as i32;
        (h, 0)
    } else {
        return None;
    };
    Some(sign * (h * 3600 + m * 60))
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
    fn test_case_insensitivity() {
        assert_eq!(parse_duration("1H30M").unwrap(), Duration::from_secs(5400));
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
    fn test_parse_datetime_timestamp() {
        let now = UNIX_EPOCH + Duration::from_secs(1000);
        let target = parse_datetime("@2000", now).unwrap();
        assert_eq!(target.duration_since(UNIX_EPOCH).unwrap().as_secs(), 2000);
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
    fn test_parse_time_utc() {
        let (h, _m, _s, offset) = parse_time("23:59Z").unwrap();
        assert_eq!(offset, Some(0));
        assert_eq!(h, Hour24(23));

        let (h, _m, _s, offset) = parse_time("12:00 UTC").unwrap();
        assert_eq!(offset, Some(0));
        assert_eq!(h, Hour24(12));

        let (_h, _m, _s, offset) = parse_time("08:00").unwrap();
        assert_eq!(offset, None);
    }

    #[test]
    fn test_parse_time_offset() {
        let (_, _, _, offset) = parse_time("12:00+02:00").unwrap();
        assert_eq!(offset, Some(7200));

        let (_, _, _, offset) = parse_time("12:00-05:00").unwrap();
        assert_eq!(offset, Some(-18000));

        let (_, _, _, offset) = parse_time("12:00+0530").unwrap();
        assert_eq!(offset, Some(19800));
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
    fn test_parse_time_am_midnight() {
        let (h, m, _s, _o) = parse_time("12:00am").unwrap();
        assert_eq!(h, Hour24(0));
        assert_eq!(m, Minute(0));

        let (h, m, _s, _o) = parse_time("12:30am").unwrap();
        assert_eq!(h, Hour24(0));
        assert_eq!(m, Minute(30));
    }

    #[test]
    fn test_parse_time_am_morning() {
        let (h, _m, _s, _o) = parse_time("1:00am").unwrap();
        assert_eq!(h, Hour24(1));

        let (h, _m, _s, _o) = parse_time("11:59am").unwrap();
        assert_eq!(h, Hour24(11));
    }

    #[test]
    fn test_parse_time_pm_noon() {
        let (h, m, _s, _o) = parse_time("12:00pm").unwrap();
        assert_eq!(h, Hour24(12));
        assert_eq!(m, Minute(0));

        let (h, m, _s, _o) = parse_time("12:30pm").unwrap();
        assert_eq!(h, Hour24(12));
        assert_eq!(m, Minute(30));
    }

    #[test]
    fn test_parse_time_pm_evening() {
        let (h, _m, _s, _o) = parse_time("1:00pm").unwrap();
        assert_eq!(h, Hour24(13));

        let (h, _m, _s, _o) = parse_time("11:59pm").unwrap();
        assert_eq!(h, Hour24(23));
    }

    #[test]
    fn test_parse_time_am_pm_case_insensitivity() {
        let (h, _m, _s, _o) = parse_time("1:00AM").unwrap();
        assert_eq!(h, Hour24(1));

        let (h, _m, _s, _o) = parse_time("1:00Pm").unwrap();
        assert_eq!(h, Hour24(13));
    }

    #[test]
    fn test_parse_time_am_pm_with_seconds() {
        let (h, _m, s, _o) = parse_time("1:00:05pm").unwrap();
        assert_eq!(h, Hour24(13));
        assert_eq!(s, Second(5));
    }

    #[test]
    fn test_parse_time_am_pm_with_offset() {
        let (_, _, _, offset) = parse_time("1:00pm+02:00").unwrap();
        assert_eq!(offset, Some(7200));
    }

    #[test]
    fn test_parse_time_am_pm_hour_only() {
        let (h, m, _s, _o) = parse_time("6pm").unwrap();
        assert_eq!(h, Hour24(18));
        assert_eq!(m, Minute(0));

        let (h, m, _s, _o) = parse_time("12am").unwrap();
        assert_eq!(h, Hour24(0));
        assert_eq!(m, Minute(0));

        let (h, m, _s, _o) = parse_time("12PM").unwrap();
        assert_eq!(h, Hour24(12));
        assert_eq!(m, Minute(0));
    }

    #[test]
    fn test_parse_time_am_pm_invalid_hours() {
        assert!(parse_time("0:00am").is_err());
        assert!(parse_time("13:00am").is_err());
        assert!(parse_time("0:00pm").is_err());
        assert!(parse_time("13:00pm").is_err());
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
