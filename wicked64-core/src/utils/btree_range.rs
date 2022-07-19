use std::collections::BTreeMap;

#[derive(Debug, Clone)]
pub struct BTreeRange<T> {
    btree: BTreeMap<usize, T>,
}

impl<T> BTreeRange<T> {
    pub fn new() -> Self {
        Self {
            btree: BTreeMap::new(),
        }
    }

    pub fn insert(&mut self, index: usize, value: T) {
        self.btree.insert(index, value);
    }

    pub fn get_offset_value(&self, index: usize) -> Option<(usize, &T)> {
        self.btree
            .range(..=index)
            .last()
            .map(|(start, value)| (index - *start, value))
    }
    pub fn get(&self, index: usize) -> Option<&T> {
        self.get_offset_value(index).map(|(_, value)| value)
    }

    pub fn get_offset_value_mut(&mut self, index: usize) -> Option<(usize, &mut T)> {
        self.btree
            .range_mut(..=index)
            .last()
            .map(|(start, value)| (index - *start, value))
    }
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.get_offset_value_mut(index).map(|(_, value)| value)
    }
}

#[macro_export]
macro_rules! map_ranges {
    ($( $range:expr => $value:expr $(,)* )* ) => {{
        let mut map = BTreeRange::new();
        $( map.insert(*$range.start(), $value); )*
        map
    }};
}
