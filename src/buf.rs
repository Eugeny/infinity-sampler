use core::mem::MaybeUninit;
use heapless::Vec;

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
    buf: Option<Vec<T, N>>,
    sample_rate: SamplingRate,
    inner_index: usize,
    outer_index: usize,
}

impl<T, const N: usize> SamplingReservoir<T, N> {
    const LOG_N: u32 = N.trailing_zeros();

    // For panic-free `x % (N / 2) == 0` operation
    const WRAPAROUND_MASK: usize = N / 2 - 1;

    /// Creates a empty reservoir, allocating an uninitialized buffer.
    /// Panics if `N` is not a power of two.
    pub const fn new() -> Self {
        assert!(N > 1);
        assert!(
            N.is_power_of_two(),
            "Buffer capacity must be a power of two"
        );
        Self {
            buf: Some(Vec::new()),
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
    pub fn len(&self) -> usize {
        unsafe { self.buf.as_ref().unwrap_unchecked() }.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Consume self and return the internal item buffer.
    pub fn into_inner(mut self) -> Vec<T, N> {
        let buf = self.buf.take();
        unsafe { buf.unwrap_unchecked() }
    }

    /// Get a view into the occupied part of the internal buffer.
    pub fn as_unordered_slice(&self) -> &[T] {
        // SAFETY: values up to fill_level are initialized
        let buf = unsafe { self.buf.as_ref().unwrap_unchecked() };
        &(buf)[..self.len()]
    }

    /// Return an iterator over
    /// the items in chronological order - *O(N)*.
    pub fn ordered_iter(&self) -> impl Iterator<Item = &T> {
        ReservoirOrderedIter2 {
            inner: ReservoirOrderedIndexIter {
                pos: 0,
                len: self.len(),
                samples_seen: self.samples_seen(),
                samples_stored: self.samples_stored(),
            },
            buf: self,
        }
    }

    /// This is irreversible and consumes the reservoir.
    pub fn into_ordered_iter(self) -> impl Iterator<Item = T> {
        OwningReservoirOrderedIter {
            inner: ReservoirOrderedIndexIter {
                pos: 0,
                len: self.len(),
                samples_seen: self.samples_seen(),
                samples_stored: self.samples_stored(),
            },
            buf: self.buf,
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

        let buf = unsafe { self.buf.as_mut().unwrap_unchecked() };
        if insert_index == buf.len() {
            let _ = buf.push(value);
        } else {
            buf[insert_index] = value;
        }
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

struct ReservoirOrderedIndexIter<const N: usize> {
    pos: usize,
    len: usize,
    samples_stored: usize,
    samples_seen: usize,
}

impl<const N: usize> ExactSizeIterator for ReservoirOrderedIndexIter<N> {}

impl<const N: usize> Iterator for ReservoirOrderedIndexIter<N> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos == self.len {
            return None;
        }

        if self.samples_seen < N {
            self.pos += 1;
            return Some(self.pos - 1);
        }

        let log = usize::BITS - ((self.samples_seen - 1) / (N - 1)).leading_zeros() - 1;
        let step_lower = 1 << log;
        let step_upper = step_lower << 1;

        let n_upper_steps = self.samples_stored % (N / 2);

        let outer_index = if self.pos < n_upper_steps {
            self.pos * step_upper
        } else if self.pos < N - n_upper_steps {
            n_upper_steps * step_upper + (self.pos - n_upper_steps) * step_lower
        } else {
            n_upper_steps * step_upper
                + (N - n_upper_steps * 2) * step_lower
                + (self.pos - (N - n_upper_steps)) * step_upper
        };
        let idx = SamplingReservoir::<(), N>::storage_index_for_outer_index(outer_index);
        self.pos += 1;

        Some(idx)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len - self.pos, Some(self.len - self.pos))
    }
}

struct ReservoirOrderedIter2<'a, T, const N: usize> {
    buf: &'a SamplingReservoir<T, N>,
    inner: ReservoirOrderedIndexIter<N>,
}

impl<T, const N: usize> ExactSizeIterator for ReservoirOrderedIter2<'_, T, N> {}

impl<'a, T, const N: usize> Iterator for ReservoirOrderedIter2<'a, T, N> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        let idx = self.inner.next()?;
        Some(&self.buf.as_unordered_slice()[idx])
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

struct OwningReservoirOrderedIter<T, const N: usize> {
    buf: Option<Vec<T, N>>,
    inner: ReservoirOrderedIndexIter<N>,
}

impl<T, const N: usize> ExactSizeIterator for OwningReservoirOrderedIter<T, N> {}

impl<T, const N: usize> OwningReservoirOrderedIter<T, N> {
    fn get_item_ref(&mut self, idx: usize) -> &mut MaybeUninit<T> {
        unsafe {
            &mut *(self.buf.as_mut().unwrap_unchecked().as_mut_ptr().add(idx)
                as *mut MaybeUninit<T>)
        }
    }
}

impl<T, const N: usize> Iterator for OwningReservoirOrderedIter<T, N> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        let idx = self.inner.next()?;
        Some(unsafe {
            core::mem::replace(self.get_item_ref(idx), MaybeUninit::uninit()).assume_init()
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<T, const N: usize> Drop for OwningReservoirOrderedIter<T, N> {
    fn drop(&mut self) {
        // Consume remaining items
        for _ in self.by_ref() {}
        core::mem::forget(self.buf.take());
    }
}

pub enum SamplingOutcome<T> {
    Consumed,
    ConsumedAndRateReduced { factor: u32 },
    Discarded(T),
}
