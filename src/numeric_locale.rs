use std::borrow::Cow;
use std::sync::OnceLock;

static DECIMAL_POINT: OnceLock<String> = OnceLock::new();

pub(crate) fn normalize(input: &str) -> Cow<'_, str> {
    let decimal_point = DECIMAL_POINT.get_or_init(decimal_point_from_environment);
    normalize_with_decimal_point(input, decimal_point)
}

fn normalize_with_decimal_point<'a>(input: &'a str, decimal_point: &str) -> Cow<'a, str> {
    if decimal_point == "." || decimal_point.is_empty() || !input.contains(decimal_point) {
        Cow::Borrowed(input)
    } else {
        Cow::Owned(input.replace(decimal_point, "."))
    }
}

#[cfg(unix)]
fn decimal_point_from_environment() -> String {
    use std::ffi::CStr;

    // Use a thread-local locale object so reading LC_NUMERIC does not mutate the
    // process-global locale. An empty locale name resolves LC_NUMERIC from the
    // standard LC_ALL/LC_NUMERIC/LANG environment variables.
    unsafe {
        let locale = libc::newlocale(libc::LC_NUMERIC_MASK, c"".as_ptr(), std::ptr::null_mut());
        if locale.is_null() {
            return ".".to_owned();
        }

        let previous = libc::uselocale(locale);
        if previous.is_null() {
            libc::freelocale(locale);
            return ".".to_owned();
        }

        let conventions = libc::localeconv();
        let decimal_point = if conventions.is_null() || (*conventions).decimal_point.is_null() {
            ".".to_owned()
        } else {
            CStr::from_ptr((*conventions).decimal_point)
                .to_string_lossy()
                .into_owned()
        };

        libc::uselocale(previous);
        libc::freelocale(locale);
        decimal_point
    }
}

#[cfg(not(unix))]
fn decimal_point_from_environment() -> String {
    ".".to_owned()
}

#[cfg(test)]
mod tests {
    use super::normalize_with_decimal_point;
    use std::borrow::Cow;

    #[test]
    fn leaves_c_locale_numbers_unchanged() {
        assert_eq!(
            normalize_with_decimal_point("1.25", "."),
            Cow::Borrowed("1.25")
        );
    }

    #[test]
    fn normalizes_locale_decimal_point() {
        assert_eq!(
            normalize_with_decimal_point("1,25e-2", ","),
            Cow::<str>::Owned("1.25e-2".to_owned())
        );
    }

    #[test]
    fn keeps_c_locale_decimal_valid_in_non_c_locale() {
        assert_eq!(
            normalize_with_decimal_point("1.25", ","),
            Cow::Borrowed("1.25")
        );
    }
}
