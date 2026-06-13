//! Conversion between Windows `FILETIME` values and Rust `SystemTime`.

use std::time::{Duration, SystemTime, UNIX_EPOCH};
use windows_sys::Win32::Foundation::FILETIME;

const WINDOWS_TO_UNIX_EPOCH_100NS: u64 = 116_444_736_000_000_000;

#[must_use]
pub const fn to_ticks(value: FILETIME) -> u64 {
    (value.dwHighDateTime as u64) << 32 | value.dwLowDateTime as u64
}

#[must_use]
pub const fn from_ticks(ticks: u64) -> FILETIME {
    FILETIME {
        dwLowDateTime: ticks as u32,
        dwHighDateTime: (ticks >> 32) as u32,
    }
}

#[must_use]
pub fn to_system_time(value: FILETIME) -> Option<SystemTime> {
    let ticks = to_ticks(value);
    let delta = ticks.checked_sub(WINDOWS_TO_UNIX_EPOCH_100NS)?;
    Some(UNIX_EPOCH + Duration::from_nanos(delta.saturating_mul(100)))
}

#[must_use]
pub fn from_system_time(value: SystemTime) -> FILETIME {
    let duration = value.duration_since(UNIX_EPOCH).unwrap_or_default();
    let ticks =
        WINDOWS_TO_UNIX_EPOCH_100NS.saturating_add(duration.as_nanos().saturating_div(100) as u64);
    from_ticks(ticks)
}

#[cfg(test)]
mod tests {
    use super::{from_system_time, to_system_time};
    use std::time::UNIX_EPOCH;

    #[test]
    fn filetime_round_trips_unix_epoch() {
        assert_eq!(
            to_system_time(from_system_time(UNIX_EPOCH)),
            Some(UNIX_EPOCH)
        );
    }
}
