use std::{
    collections::BTreeMap,
    ops::{Bound, RangeBounds},
};

#[derive(Debug, Clone)]
pub struct RangeItem<T> {
    data: T,
    end: usize,
}

impl<T> RangeItem<T> {
    pub fn new(data: T, end: usize) -> Self {
        Self { data, end }
    }
}

#[derive(Debug, Clone)]
pub struct BTreeRange<T> {
    btree: BTreeMap<usize, RangeItem<T>>,
}

impl<T> BTreeRange<T> {
    pub fn new() -> Self {
        Self {
            btree: BTreeMap::new(),
        }
    }

    pub fn insert<R>(&mut self, range: R, value: T)
    where
        R: RangeBounds<usize>,
    {
        // `start` should be an included bound
        let start = match range.start_bound().cloned() {
            Bound::Excluded(n) => n + 1,
            Bound::Included(n) => n,
            Bound::Unbounded => 0,
        };
        // `end` should be an excluded bound
        let end = match range.end_bound().cloned() {
            Bound::Excluded(n) => n,
            Bound::Included(n) => n + 1,
            Bound::Unbounded => usize::MAX,
        };

        self.btree.insert(start, RangeItem::new(value, end));
    }

    pub fn get_offset_value(&self, index: usize) -> Option<(usize, &T)> {
        self.btree
            .range(..=index)
            .last()
            .map(|(start, RangeItem { data, end })| {
                if index < *end {
                    Some((index - *start, data))
                } else {
                    None
                }
            })
            .flatten()
    }
    pub fn get_offset_value_mut(&mut self, index: usize) -> Option<(usize, &mut T)> {
        self.btree
            .range_mut(..=index)
            .last()
            .map(|(start, RangeItem { data, end })| {
                if index < *end {
                    Some((index - *start, data))
                } else {
                    None
                }
            })
            .flatten()
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        self.get_offset_value(index).map(|(_, value)| value)
    }
    pub fn get_mut(&mut self, index: usize) -> Option<&mut T> {
        self.get_offset_value_mut(index).map(|(_, value)| value)
    }
    pub fn get_exact(&self, index: usize) -> Option<&T> {
        self.btree.get(&index).map(|value| &value.data)
    }

    pub(crate) fn retain<F>(&mut self, mut f: F)
    where
        F: FnMut((usize, usize), &mut T) -> bool,
    {
        self.btree.retain(|k, v| f((*k, v.end), &mut v.data));
    }
}

#[macro_export]
macro_rules! map_ranges {
    ($( $range:expr => $value:expr $(,)* )* ) => {{
        let mut map = BTreeRange::new();
        $( map.insert($range.clone(), $value); )*
        map
    }};
}
