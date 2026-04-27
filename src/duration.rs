use std::io::{self, Write};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Seconds(pub u64);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Minutes(pub u64);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Hours(pub u64);

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Days(pub u64);

impl From<Minutes> for Seconds {
    fn from(m: Minutes) -> Self {
        Seconds(m.0 * 60)
    }
}

impl From<Hours> for Seconds {
    fn from(h: Hours) -> Self {
        Seconds(h.0 * 3600)
    }
}

impl From<Days> for Seconds {
    fn from(d: Days) -> Self {
        Seconds(d.0 * 86400)
    }
}

impl Seconds {
    pub fn checked_add(self, other: Self) -> Option<Self> {
        self.0.checked_add(other.0).map(Seconds)
    }

    pub fn write_human(&self, w: &mut impl Write) -> io::Result<()> {
        if self.0 == 0 {
            return w.write_all(b"0s");
        }

        let mut s = self.0;
        let days = s / 86400;
        s %= 86400;
        let hours = s / 3600;
        s %= 3600;
        let minutes = s / 60;
        let seconds = s % 60;

        if days > 0 {
            write_int(w, days)?;
            w.write_all(b"d")?;
        }
        if hours > 0 {
            write_int(w, hours)?;
            w.write_all(b"h")?;
        }
        if minutes > 0 {
            write_int(w, minutes)?;
            w.write_all(b"m")?;
        }
        if seconds > 0 {
            write_int(w, seconds)?;
            w.write_all(b"s")?;
        }
        Ok(())
    }
}

fn write_int(w: &mut impl Write, mut n: u64) -> io::Result<()> {
    if n == 0 {
        return w.write_all(b"0");
    }
    let mut buf = [0u8; 20];
    let mut pos = 20;
    while n > 0 {
        pos -= 1;
        buf[pos] = b'0' + (n % 10) as u8;
        n /= 10;
    }
    w.write_all(&buf[pos..])
}

impl std::fmt::Display for Seconds {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut buf = Vec::new();
        let _ = self.write_human(&mut buf);
        write!(f, "{}", String::from_utf8_lossy(&buf))
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct PollInterval(pub Seconds);

impl Default for PollInterval {
    fn default() -> Self {
        PollInterval(Seconds(1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_human() {
        assert_eq!(Seconds(0).to_string(), "0s");
        assert_eq!(Seconds(60).to_string(), "1m");
        assert_eq!(Seconds(65).to_string(), "1m5s");
        assert_eq!(Seconds(3600).to_string(), "1h");
        assert_eq!(Seconds(3661).to_string(), "1h1m1s");
        assert_eq!(Seconds(86400).to_string(), "1d");
        assert_eq!(Seconds(86400 + 3600 + 60 + 1).to_string(), "1d1h1m1s");
    }
}
