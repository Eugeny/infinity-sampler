#![doc = include_str!("../README.md")]

mod buf;
mod item;
mod iter;
mod rate;

pub use buf::{RawReservoir, SamplingOutcome, SamplingReservoir, ReservoirOrderedIter};
pub use item::{InitializedItem, Item};
pub use iter::{InfinitySamplerIndexer, ReverseInfinitySamplerIndexer};
pub use rate::SamplingRate;

#[cfg(test)]
mod tests;
