#![no_std]
#![doc = include_str!("../README.md")]

mod buf;
mod rate;

#[cfg(doc)]
pub mod math;

pub use buf::{SamplingOutcome, SamplingReservoir};
pub use rate::SamplingRate;

#[cfg(test)]
#[macro_use]
extern crate std;

#[cfg(test)]
mod tests;

/// `heapless` re-export
pub use heapless;
