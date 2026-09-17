#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundedVec<T, const MIN: usize, const MAX: usize>(Vec<T>);

impl<T, const MIN: usize, const MAX: usize> BoundedVec<T, MIN, MAX> {
    pub fn new(values: Vec<T>) -> Option<Self> {
        (MIN <= values.len() && values.len() <= MAX).then_some(Self(values))
    }

    pub fn as_slice(&self) -> &[T] {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IncludedId(i64);

impl IncludedId {
    pub const MIN: i64 = 1;
    pub const MAX: i64 = 65535;

    pub const fn new(value: i64) -> Option<Self> {
        if value >= Self::MIN && value <= Self::MAX {
            Some(Self(value))
        } else {
            None
        }
    }

    pub const fn get(self) -> i64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IncludedQuality {
    Unknown,
    Good,
    Bad,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordFirstInSource {
    pub id: IncludedId,
    pub quality: IncludedQuality,
}
