use crate::datetime::{DateTime, to_unix_timestamp};

pub fn utc_offset_seconds() -> i32 {
    imp::utc_offset_seconds()
}

pub fn utc_offset_seconds_at(dt: DateTime) -> Option<i32> {
    imp::utc_offset_seconds_at(dt)
}

pub fn utc_offset_hours() -> i32 {
    utc_offset_seconds() / 3600
}

#[cfg(unix)]
mod imp {
    use super::*;

    pub fn utc_offset_seconds() -> i32 {
        let mut now = 0; // time_t/i64 (use type-inference for infinite compatibility :))
        // SAFETY: libc::time is safe when passed a valid pointer or mut reference to a time_t.
        if unsafe { libc::time(&mut now) } == -1 {
            return 0;
        }

        let mut tm: libc::tm = unsafe { std::mem::zeroed() };
        // SAFETY: libc::localtime_r is thread-safe and we pass a valid pointer to tm.
        if unsafe { libc::localtime_r(&now, &mut tm) }.is_null() {
            return 0;
        }

        #[cfg(any(
            target_os = "linux",
            target_os = "macos",
            target_os = "freebsd",
            target_os = "openbsd",
            target_os = "netbsd"
        ))]
        {
            tm.tm_gmtoff as i32
        }

        #[cfg(not(any(
            target_os = "linux",
            target_os = "macos",
            target_os = "freebsd",
            target_os = "openbsd",
            target_os = "netbsd"
        )))]
        {
            let mut utc_tm: libc::tm = unsafe { std::mem::zeroed() };
            // SAFETY: libc::gmtime_r is thread-safe and we pass a valid pointer to utc_tm.
            if unsafe { libc::gmtime_r(&now, &mut utc_tm) }.is_null() {
                return 0;
            }

            let mut local_copy = tm;
            let mut utc_copy = utc_tm;

            local_copy.tm_isdst = -1;
            utc_copy.tm_isdst = -1;

            // SAFETY: libc::mktime is safe when passed a valid mutable reference to a tm struct.
            let local_time = unsafe { libc::mktime(&mut local_copy) };
            // SAFETY: libc::mktime is safe when passed a valid mutable reference to a tm struct.
            let utc_time = unsafe { libc::mktime(&mut utc_copy) };

            if local_time == -1 || utc_time == -1 {
                return 0;
            }

            (local_time - utc_time) as i32
        }
    }

    pub fn utc_offset_seconds_at(dt: DateTime) -> Option<i32> {
        let mut tm: libc::tm = unsafe { std::mem::zeroed() };
        tm.tm_year = dt.year.0.checked_sub(1900)?;
        tm.tm_mon = i32::from(dt.month.0).checked_sub(1)?;
        tm.tm_mday = i32::from(dt.day.0);
        tm.tm_hour = i32::from(dt.hour.0);
        tm.tm_min = i32::from(dt.minute.0);
        tm.tm_sec = i32::from(dt.second.0);
        tm.tm_isdst = -1;

        // SAFETY: libc::mktime is safe when passed a valid mutable reference to a tm struct.
        // Setting tm_isdst to -1 asks the C runtime to determine DST for this local datetime.
        let timestamp = unsafe { libc::mktime(&mut tm) };
        if timestamp == -1 {
            return None;
        }

        // mktime may normalize nonexistent local times (for example, during a spring-forward
        // transition). Treat those as invalid rather than silently changing the requested time.
        if tm.tm_year != dt.year.0 - 1900
            || tm.tm_mon != i32::from(dt.month.0) - 1
            || tm.tm_mday != i32::from(dt.day.0)
            || tm.tm_hour != i32::from(dt.hour.0)
            || tm.tm_min != i32::from(dt.minute.0)
            || tm.tm_sec != i32::from(dt.second.0)
        {
            return None;
        }

        let local_as_utc = i128::from(to_unix_timestamp(dt));
        let offset = local_as_utc.checked_sub(timestamp as i128)?;
        i32::try_from(offset).ok()
    }
}

#[cfg(windows)]
mod imp {
    use super::*;
    use windows_sys::Win32::Foundation::SYSTEMTIME;
    use windows_sys::Win32::System::Time::{
        DYNAMIC_TIME_ZONE_INFORMATION, GetDynamicTimeZoneInformation, GetTimeZoneInformation,
        TIME_ZONE_ID_INVALID, TzSpecificLocalTimeToSystemTimeEx,
    };

    pub fn utc_offset_seconds() -> i32 {
        let mut dynamic_info = unsafe { std::mem::zeroed() };
        // SAFETY: GetDynamicTimeZoneInformation is safe when passed a valid pointer to DYNAMIC_TIME_ZONE_INFORMATION.
        let id = unsafe { GetDynamicTimeZoneInformation(&mut dynamic_info) };

        if id != TIME_ZONE_ID_INVALID {
            let bias = dynamic_info.Bias
                + if id == 2 {
                    dynamic_info.DaylightBias
                } else {
                    dynamic_info.StandardBias
                };
            return -bias * 60;
        }

        let mut info = unsafe { std::mem::zeroed() };
        // SAFETY: GetTimeZoneInformation is safe when passed a valid pointer to TIME_ZONE_INFORMATION.
        let id = unsafe { GetTimeZoneInformation(&mut info) };
        if id != TIME_ZONE_ID_INVALID {
            let bias = info.Bias
                + if id == 2 {
                    info.DaylightBias
                } else {
                    info.StandardBias
                };
            return -bias * 60;
        }

        0
    }

    pub fn utc_offset_seconds_at(dt: DateTime) -> Option<i32> {
        let local = SYSTEMTIME {
            wYear: u16::try_from(dt.year.0).ok()?,
            wMonth: u16::from(dt.month.0),
            wDayOfWeek: 0,
            wDay: u16::from(dt.day.0),
            wHour: u16::from(dt.hour.0),
            wMinute: u16::from(dt.minute.0),
            wSecond: u16::from(dt.second.0),
            wMilliseconds: 0,
        };
        let mut utc: SYSTEMTIME = unsafe { std::mem::zeroed() };

        // SAFETY: A null timezone pointer selects the current system timezone. Both SYSTEMTIME
        // pointers are valid for the duration of the call.
        if unsafe {
            TzSpecificLocalTimeToSystemTimeEx(
                std::ptr::null::<DYNAMIC_TIME_ZONE_INFORMATION>(),
                &local,
                &mut utc,
            )
        } == 0
        {
            return None;
        }

        let utc_dt = DateTime {
            year: crate::datetime::Year(i32::from(utc.wYear)),
            month: crate::datetime::Month(u8::try_from(utc.wMonth).ok()?),
            day: crate::datetime::Day(u8::try_from(utc.wDay).ok()?),
            hour: crate::datetime::Hour24(u8::try_from(utc.wHour).ok()?),
            minute: crate::datetime::Minute(u8::try_from(utc.wMinute).ok()?),
            second: crate::datetime::Second(u8::try_from(utc.wSecond).ok()?),
        };
        let local_as_utc = to_unix_timestamp(dt);
        let utc_timestamp = to_unix_timestamp(utc_dt);
        let offset = i128::from(local_as_utc).checked_sub(i128::from(utc_timestamp))?;
        i32::try_from(offset).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Local, Offset};

    #[test]
    fn test_matches_chrono() {
        assert_eq!(
            utc_offset_seconds(),
            Local::now().offset().fix().local_minus_utc()
        );
    }

    #[test]
    fn test_range() {
        let o = utc_offset_seconds();
        assert!(o.abs() <= 14 * 3600);
        assert_eq!(o % 60, 0);
    }

    #[test]
    fn test_hours_consistency() {
        assert_eq!(utc_offset_hours(), utc_offset_seconds() / 3600);
    }

    #[test]
    fn test_stability() {
        assert_eq!(utc_offset_seconds(), utc_offset_seconds());
    }
}
