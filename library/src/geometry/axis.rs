use strum_macros::EnumCount;

#[derive(EnumCount, Copy, Clone, Default, Debug, PartialEq)]
pub enum Axis {
    #[default]
    X,
    Y,
    Z,
}
