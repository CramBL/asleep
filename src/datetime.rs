#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Year(pub i32);
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Month(pub u8);
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Day(pub u8);
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hour24(pub u8);
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Minute(pub u8);
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Second(pub u8);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DateTime {
    pub year: Year,
    pub month: Month,
    pub day: Day,
    pub hour: Hour24,
    pub minute: Minute,
    pub second: Second,
}

impl Month {
    pub fn new(m: u8) -> Option<Self> {
        if (1..=12).contains(&m) {
            Some(Month(m))
        } else {
            None
        }
    }
}

impl Day {
    pub fn new(d: u8, year: Year, month: Month) -> Option<Self> {
        let max_days = match month.0 {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 => {
                if is_leap_year(year.0) {
                    29
                } else {
                    28
                }
            }
            _ => return None,
        };
        if (1..=max_days).contains(&d) {
            Some(Day(d))
        } else {
            None
        }
    }
}

impl Hour24 {
    pub fn new(h: u8) -> Option<Self> {
        if h < 24 { Some(Hour24(h)) } else { None }
    }
}

impl Minute {
    pub fn new(m: u8) -> Option<Self> {
        if m < 60 { Some(Minute(m)) } else { None }
    }
}

impl Second {
    pub fn new(s: u8) -> Option<Self> {
        if s < 60 { Some(Second(s)) } else { None }
    }
}

pub fn is_leap_year(y: i32) -> bool {
    (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0)
}

/// Converts DateTime to Unix timestamp (seconds since 1970-01-01 00:00:00 UTC)
pub fn to_unix_timestamp(dt: DateTime) -> u64 {
    let mut days = 0i64;
    for y in 1970..dt.year.0 {
        days += if is_leap_year(y) { 366 } else { 365 };
    }

    let month_days = [0, 31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    for &md in month_days.iter().take(dt.month.0 as usize).skip(1) {
        days += md as i64;
    }
    if dt.month.0 > 2 && is_leap_year(dt.year.0) {
        days += 1;
    }

    days += (dt.day.0 - 1) as i64;

    let total_seconds =
        days * 86400 + (dt.hour.0 as i64 * 3600) + (dt.minute.0 as i64 * 60) + (dt.second.0 as i64);

    if total_seconds < 0 {
        0
    } else {
        total_seconds as u64
    }
}

pub fn from_unix_timestamp(mut ts: u64) -> DateTime {
    let second = Second((ts % 60) as u8);
    ts /= 60;
    let minute = Minute((ts % 60) as u8);
    ts /= 60;
    let hour = Hour24((ts % 24) as u8);
    ts /= 24;

    let mut days = ts as i32;
    let mut year = 1970;
    loop {
        let year_days = if is_leap_year(year) { 366 } else { 365 };
        if days < year_days {
            break;
        }
        days -= year_days;
        year += 1;
    }

    let month_days = [0, 31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let mut month = 1;
    loop {
        let mut md = month_days[month];
        if month == 2 && is_leap_year(year) {
            md += 1;
        }
        if days < md {
            break;
        }
        days -= md;
        month += 1;
    }

    DateTime {
        year: Year(year),
        month: Month(month as u8),
        day: Day((days + 1) as u8),
        hour,
        minute,
        second,
    }
}
