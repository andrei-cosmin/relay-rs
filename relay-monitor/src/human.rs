use std::time::Duration;

use bytesize::ByteSize;

pub struct Human;

impl Human {
    pub fn duration(milliseconds: u64) -> String {
        humantime::format_duration(Duration::from_millis(milliseconds)).to_string()
    }

    pub fn size(bytes: u64) -> String {
        ByteSize(bytes).to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durations_read_like_humantime() {
        assert_eq!(Human::duration(999), "999ms");
        assert_eq!(Human::duration(1_250), "1s 250ms");
        assert_eq!(Human::duration(65_000), "1m 5s");
        assert_eq!(Human::duration(3_725_000), "1h 2m 5s");
    }

    #[test]
    fn sizes_use_binary_units() {
        assert_eq!(Human::size(2), "2 B");
        assert_eq!(Human::size(10_035), "9.8 KiB");
        assert_eq!(Human::size(536_887), "524.3 KiB");
        assert_eq!(Human::size(1_258_291), "1.2 MiB");
    }
}
