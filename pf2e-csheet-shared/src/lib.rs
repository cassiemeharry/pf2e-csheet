#[macro_use]
extern crate log;

#[cfg(test)]
#[macro_use]
extern crate pretty_assertions;

#[macro_use]
mod macros;

pub mod bonuses;
pub mod calc;
mod character;
pub mod choices;
mod common;
pub mod cond;
pub mod effects;
pub mod items;
pub mod messages;
pub mod parsers;
pub mod stats;
pub mod storage;

pub use character::*;
pub use common::*;

pub(crate) fn is_default<T: Default + PartialEq>(obj: &T) -> bool {
    let default = T::default();
    &default == obj
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
