use std::fmt;
use std::fmt::{Display, Formatter};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash,)]
pub struct ObjectUid(pub u32);

impl From<usize> for ObjectUid {
    fn from(value: usize) -> Self {
        ObjectUid(value as u32)
    }
}

impl From<u32> for ObjectUid {
    fn from(value: u32) -> Self {
        ObjectUid(value)
    }
}

impl Display for ObjectUid {
    fn fmt(&self, formatter: &mut Formatter) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}