use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct Page<T> {
    pub number: usize,
    pub size: usize,
    pub total: u64,
    pub items: Vec<T>,
}

impl<T> Page<T> {
    pub fn pages(&self) -> usize {
        if self.size == 0 {
            return 1;
        }
        usize::try_from(self.total)
            .unwrap_or(usize::MAX)
            .div_ceil(self.size)
            .max(1)
    }
}

impl<T> Default for Page<T> {
    fn default() -> Self {
        Self {
            number: 0,
            size: 0,
            total: 0,
            items: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Page;

    fn page(size: usize, total: u64) -> Page<u8> {
        Page {
            size,
            total,
            ..Page::default()
        }
    }

    #[test]
    fn pages_round_up() {
        assert_eq!(page(20, 45).pages(), 3);
        assert_eq!(page(20, 40).pages(), 2);
    }

    #[test]
    fn an_empty_or_sizeless_page_still_counts_as_one() {
        assert_eq!(page(20, 0).pages(), 1);
        assert_eq!(page(0, 45).pages(), 1);
    }
}
