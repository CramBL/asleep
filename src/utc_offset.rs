pub fn utc_offset_seconds() -> i32 {
    imp::utc_offset_seconds()
}

pub fn utc_offset_hours() -> i32 {
    utc_offset_seconds() / 3600
}

#[cfg(unix)]
mod imp {
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
}

#[cfg(windows)]
mod imp {
    use windows_sys::Win32::System::Time::{
        GetDynamicTimeZoneInformation, GetTimeZoneInformation, TIME_ZONE_ID_INVALID,
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
