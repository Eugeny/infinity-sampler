#![no_std]
#![doc = include_str!("../README.md")]

mod buf;
mod item;
mod rate;

#[cfg(doc)]
pub mod math;

pub use buf::{SamplingOutcome, SamplingReservoir, ReservoirOrderedIter};
pub use item::{InitializedItem, Item};
pub use rate::SamplingRate;

#[cfg(test)]
#[macro_use]
extern crate std;

#[cfg(test)]
mod tests;
