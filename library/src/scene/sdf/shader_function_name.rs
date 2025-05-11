use derive_more::Display;
use crate::scene::sdf::named_sdf::UniqueName;

#[derive(Clone, Debug, Eq, PartialEq, Hash, Display)]
#[display("{}", _0)]
pub(in crate::scene::sdf) struct FunctionName(pub String);

impl From<&UniqueName> for FunctionName {
    #[must_use]
    fn from(value: &UniqueName) -> Self {
        FunctionName(format!("sdf_{}", value.0))
    }
}