
#[macro_export]
macro_rules! panic_if_failed {
    ($expr:expr, $($arg:tt)+) => {
        if !$expr {
            panic!($($arg)+);
        }
    };
}