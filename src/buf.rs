use crate::item::{InitializedItem, Item};
use core::hint::unreachable_unchecked;
use core::num::NonZeroUsize;

pub use crate::rate::SamplingRate;

/// # Infinity Sampler
///
/// See the [top-level doc](crate) for an example.
///
/// The sampling rate gets halved after every `N/2` stored values, which is the same
/// as every `N*2^X` values observed by the sampler.
///
/// Feed the values into the reservoir using [sample()](Self::sample) and then
/// turn it into an ordered iterator with [into_ordered_iter()](Self::into_ordered_iter).
///
/// The buffer size must be a power of two.
#[derive(Clone)]
pub struct SamplingReservoir<T, const N: usize> {
    buf: Option<[Item<T>; N]>,
    fill_level: usize,
    sample_rate: SamplingRate,
    inner_index: usize,
    outer_index: usize,
}

impl<T, const N: usize> SamplingReservoir<T, N> {
    const EMPTY: Item<T> = Item::empty();
    const LOG_N: u32 = N.trailing_zeros();

    // For panic-free `x % (N / 2) == 0` operation
    const WRAPAROUND_MASK: usize = N / 2 - 1;

    /// Creates a empty reservoir, allocating an uninitialized buffer.
    pub const fn new() -> Self {
        assert!(N > 1);
        assert!(
            N.is_power_of_two(),
            "Buffer capacity must be a power of two"
        );
        Self {
            buf: Some([Self::EMPTY; N]),
            fill_level: 0,
            sample_rate: SamplingRate::new(1),
            inner_index: 0,
            outer_index: 0,
        }
    }

    /// Returns N, the capacity of the internal buffer.
    pub const fn capacity(&self) -> usize {
        N
    }

    /// Get the number of currently stored items. Can be from 0 to N-1 and never decreases.
    pub const fn len(&self) -> usize {
        self.fill_level
    }

    /// Consume self and return the internal components: item buffer and iterator state.
    pub fn into_inner(mut self) -> [Item<T>; N] {
        let buf = self.buf.take();
        unsafe { buf.unwrap_unchecked() }
    }

    /// Get a view into the occupied part of the internal buffer.
    pub fn as_unordered_slice(&self) -> &[InitializedItem<T>] {
        // SAFETY: values up to fill_level are initialized
        unsafe {
            &*(&self.buf.as_ref().unwrap_unchecked()[..self.fill_level] as *const [Item<T>]
                as *const [InitializedItem<T>])
        }
    }

    /// Sort the reservoir in-place, and return an iterator over
    /// the items in chronological order - *O(N\*log(n))*.
    ///
    /// This is irreversible and consumes the reservoir.
    pub fn into_ordered_iter(mut self) -> ReservoirOrderedIter<T, N> {
        unsafe { self.buf.as_mut().unwrap_unchecked() }.sort_unstable_by_key(|x| {
            match x.insertion_index {
                Some(x) => x.into(),
                None => usize::MAX,
            }
        });
        ReservoirOrderedIter {
            buf: unsafe { self.buf.take().unwrap_unchecked() },
            len: self.len(),
            pos: 0,
        }
    }
    /// Returns a reference to the current sampling rate.
    pub fn sampling_rate(&self) -> &SamplingRate {
        &self.sample_rate
    }

    /// Returns the total number of samples written into the buffer since the beginning.
    pub fn samples_stored(&self) -> usize {
        self.inner_index
    }

    /// Returns the total number of samples observed by the sampler since the beginning.
    pub fn samples_seen(&self) -> usize {
        self.outer_index
    }

    pub(crate) fn storage_index_for_outer_index(outer_index: usize) -> usize {
        if N - 1 == 0 {
            unsafe {
                unreachable_unchecked();
            }
        }
        match outer_index {
            0 => 0,
            i => ((i - 1) % (N - 1)) + 1,
        }
    }

    #[allow(dead_code)]
    pub(crate) fn should_sample(outer_index: usize) -> bool {
        let significant_bits = usize::BITS - outer_index.leading_zeros();
        let counter_bits = significant_bits.saturating_sub(Self::LOG_N);
        let mask = (1 << counter_bits) - 1;
        outer_index & mask == 0
    }

    /// Unconditionally stores a value in the reservoir.
    pub(crate) fn write_at_outer_index(&mut self, outer_index: usize, value: T) {
        let insert_index = Self::storage_index_for_outer_index(outer_index);

        unsafe {
            self.buf.as_mut().unwrap_unchecked()[insert_index]
                .write(NonZeroUsize::new_unchecked(outer_index + 1), value);
        }

        self.fill_level = self.fill_level.min(N - 1) + 1;
    }

    /// Observe a value and possibly store it - *O(1)*.
    ///
    /// Performs a sampling "step", consuming the value and storing it into the buffer,
    /// or returning it back if it's discarded due to the sampling rate.
    #[inline(never)]
    pub fn sample(&mut self, value: T) -> SamplingOutcome<T> {
        self.outer_index += 1;
        if !self.sample_rate.step() {
            return SamplingOutcome::Discarded(value);
        }
        let mut result = SamplingOutcome::Consumed;

        if self.inner_index >= N && (self.inner_index - N) & Self::WRAPAROUND_MASK == 0 {
            self.sample_rate.div(2);
            result = SamplingOutcome::ConsumedAndRateReduced { factor: 2 };
        }
        self.inner_index += 1;
        self.write_at_outer_index(self.outer_index - 1, value);
        result
    }
}

/// Chronological iterator over stored items.
///
/// This struct is returned by the `into_ordered_iter` method of both reservoirs.
/// The entire buffer of a reservoir is moved into this struct.
pub struct ReservoirOrderedIter<T, const N: usize> {
    buf: [Item<T>; N],
    len: usize,
    pos: usize,
}

impl<T, const N: usize> ReservoirOrderedIter<T, N> {
    /// Returns the total number of items.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns a view of the items in chronological order.
    pub fn as_slice(&self) -> &[InitializedItem<T>] {
        // SAFETY: values up to len are initialized
        unsafe { &*(&self.buf[..self.len] as *const [Item<T>] as *const [InitializedItem<T>]) }
    }
}

impl<T, const N: usize> ExactSizeIterator for ReservoirOrderedIter<T, N> {}

impl<T, const N: usize> Iterator for ReservoirOrderedIter<T, N> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos == self.len {
            return None;
        }

        let idx = self.pos;
        self.pos += 1;

        // SAFETY: values up to len are initialized
        Some(unsafe { self.buf[idx].take_unchecked() })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len - self.pos, Some(self.len - self.pos))
    }
}

pub enum SamplingOutcome<T> {
    Consumed,
    ConsumedAndRateReduced { factor: u32 },
    Discarded(T),
}
