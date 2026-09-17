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
pub struct TrackId(i64);

impl TrackId {
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
pub enum TrackQuality {
    Unknown,
    Tentative,
    Confirmed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Track {
    pub id: TrackId,
    pub quality: TrackQuality,
    pub callsign: Option<String>,
    pub sensor_ids: BoundedVec<i64, 0, 8>,
}
