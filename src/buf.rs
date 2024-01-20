use delegate::delegate;

use crate::item::{InitializedItem, Item};
use crate::InfinitySamplerIndexer;
use core::num::NonZeroUsize;

pub use crate::rate::SamplingRate;

/// # Underlying non-sampling reservoir
///
/// You probably want to use the [SamplingReservoir] wrapper.
/// See there for documentation of most methods as well.
pub struct RawReservoir<T, const N: usize> {
    // Option to allow moving out
    buf: Option<[Item<T>; N]>,
    iter: InfinitySamplerIndexer<N>,
    fill_level: usize,
}

pub enum SamplingOutcome<T> {
    Consumed,
    Discarded(T),
}

impl<T: Clone, const N: usize> Clone for RawReservoir<T, N> {
    fn clone(&self) -> Self {
        Self {
            buf: self.buf.clone(),
            iter: self.iter.clone(),
            fill_level: self.fill_level,
        }
    }
}

impl<T, const N: usize> RawReservoir<T, N> {
    const INDEXING_LOOP_SIZE: usize = N * 2;
    const IN_GROUP_BITS: u32 = (N / 2).trailing_zeros();
    const EMPTY: Item<T> = Item::empty();

    pub const fn new() -> Self {
        assert!(
            N.is_power_of_two(),
            "Buffer capacity must be a power of two"
        );
        Self {
            buf: Some([Self::EMPTY; N]),
            iter: InfinitySamplerIndexer::new(),
            fill_level: 0,
        }
    }

    pub const fn capacity(&self) -> usize {
        N
    }

    pub const fn len(&self) -> usize {
        self.fill_level
    }

    pub fn into_inner(mut self) -> ([Item<T>; N], InfinitySamplerIndexer<N>) {
        let buf = self.buf.take();
        (unsafe { buf.unwrap_unchecked() }, self.iter)
    }

    pub fn as_unordered_slice(&self) -> &[InitializedItem<T>] {
        // SAFETY: values up to fill_level are initialized
        unsafe {
            &*(&self.buf.as_ref().unwrap_unchecked()[..self.fill_level] as *const [Item<T>]
                as *const [InitializedItem<T>])
        }
    }

    #[inline(always)]
    #[allow(dead_code)]
    const fn _pattern_base_exp_for_in_loop_index(in_loop_index: usize) -> usize {
        in_loop_index >> Self::IN_GROUP_BITS
    }

    #[inline(always)]
    #[allow(dead_code)]
    pub(crate) const fn _pattern_base_for_in_loop_index(in_loop_index: usize) -> usize {
        1 << (Self::_pattern_base_exp_for_in_loop_index(in_loop_index) as u32)
    }

    #[inline(always)]
    #[allow(dead_code)]
    pub(crate) const fn _pattern_index_for_in_loop_index(mut in_loop_index: usize) -> usize {
        // Consider that storage indexes loop over and over in groups:
        // For N=16, they are:
        // [0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15] - initial insertion, then:
        // 1 3 5 7 9 11 13 15
        // 2 6 10 14
        // 3 7 11 15
        // 4 12
        // 5 13
        // 6 14
        // 7 15
        // (8x 1-index groups): 8 9 10 11 12 13 14 15
        //
        // i.e. for i, there's i groups of (N/2/i) numbers with an offset of i between them
        // and each groups starts further and further from the left

        in_loop_index = in_loop_index % Self::INDEXING_LOOP_SIZE;

        let pattern_base_exp = Self::_pattern_base_exp_for_in_loop_index(in_loop_index);
        let pattern_base = Self::_pattern_base_for_in_loop_index(in_loop_index);

        pattern_base
            + ((in_loop_index >> (Self::IN_GROUP_BITS as usize - pattern_base_exp))
                & ((1 << pattern_base_exp) - 1))
    }

    pub(crate) fn _naive_storage_index(inner_index: usize) -> usize {
        if inner_index < N {
            inner_index
        } else {
            let in_loop_index = (inner_index - N) % Self::INDEXING_LOOP_SIZE;

            // Simple implementation, but slow
            let mut pattern_index = 1;
            let mut group_size = N / 2;
            let mut group_step = 2;
            let mut group_instances = 1;

            let mut remainder = in_loop_index;
            'outer: loop {
                for _ in 0..group_instances {
                    if remainder < group_size {
                        break 'outer;
                    }
                    pattern_index += 1;
                    remainder -= group_size;
                }
                group_size = (group_size / 2).max(1);
                group_step *= 2;
                group_instances *= 2;
            }

            pattern_index + remainder * group_step
        }
    }

    pub(crate) const fn _optimized_storage_index(inner_index: usize) -> usize {
        if inner_index < N {
            inner_index
        } else {
            let in_loop_index = (inner_index - N) % Self::INDEXING_LOOP_SIZE;
            let pattern_index = Self::_pattern_index_for_in_loop_index(in_loop_index);

            let left_offset = pattern_index;

            //extr
            let pattern_base_exp = Self::_pattern_base_exp_for_in_loop_index(in_loop_index);
            let pattern_base = Self::_pattern_base_for_in_loop_index(in_loop_index);
            let group_size = N / 2 / pattern_base;
            //extr

            let pattern_offset = in_loop_index % group_size;

            let pattern_step = 1 << (pattern_base_exp as u32 + 1);
            let idx = left_offset + pattern_offset * pattern_step;

            // println!("pbe {pattern_base_exp} po {pattern_offset} lo {left_offset} po {pattern_offset} ps {pattern_step} idx {}", idx);
            debug_assert!(idx < N);
            idx
        }
    }

    /// Unconditionally stores a value in the reservoir.
    pub fn write(&mut self, value: T) {
        let iter_position = self.iter.position();

        // SAFETY: the iterator never ends
        let insert_index = unsafe { self.iter.next().unwrap_unchecked() };

        unsafe {
            self.buf.as_mut().unwrap_unchecked()[insert_index]
                .write(NonZeroUsize::new_unchecked(iter_position + 1), value);
        }

        self.fill_level = self.fill_level.min(N - 1) + 1;
    }

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

/// # Infinity Sampler
///
/// See the [top-level doc](crate) for an example.
///
/// This struct wraps a [RawReservoir] adding an autoscaling sampler in front of it.
/// The sampling rate gets halved after every `N/2` stored values, which is the same
/// as every `N*2^X` values observed by the sampler.
///
/// Feed the values into the reservoir using [sample()](Self::sample) and then
/// turn it into an ordered iterator with [into_ordered_iter()](Self::into_ordered_iter).
///
/// The buffer size must be a power of two.
#[derive(Clone)]
pub struct SamplingReservoir<T, const N: usize> {
    inner: RawReservoir<T, N>,
    sample_rate: SamplingRate,
}

impl<T, const N: usize> SamplingReservoir<T, N> {
    delegate! {
        to self.inner {
            /// Returns N, the capacity of the internal buffer.
            pub const fn capacity(&self) -> usize;

            /// Get the number of currently stored items. Can be from 0 to N-1 and never decreases.
            pub const fn len(&self) -> usize;

            /// Consume self and return the internal components: item buffer and iterator state.
            pub fn into_inner(self) -> ([Item<T>; N], InfinitySamplerIndexer<N>);

            /// Get a view into the occupied part of the internal buffer.
            pub fn as_unordered_slice(&self) -> &[InitializedItem<T>];

            /// Sort the reservoir in-place, and return an iterator over
            /// the items in chronological order - *O(N\*log(n))*.
            ///
            /// This is irreversible and consumes the reservoir.
            pub fn into_ordered_iter(self) -> ReservoirOrderedIter<T, N>;
        }
    }

    /// Creates a empty reservoir, allocating an uninitialized buffer.
    pub const fn new() -> Self {
        Self {
            inner: RawReservoir::new(),
            sample_rate: SamplingRate::new(1),
        }
    }

    /// Returns a reference to the current sampling rate.
    pub fn sampling_rate(&self) -> &SamplingRate {
        &self.sample_rate
    }

    /// Observe a value and possibly store it - *O(1)*.
    ///
    /// Performs a sampling "step", consuming the value and storing it into the buffer,
    /// or returning it back if it's discarded due to the sampling rate.
    pub fn sample(&mut self, value: T) -> SamplingOutcome<T> {
        if self.sample_rate.step() {
            if self.inner.iter.position() >= N && (self.inner.iter.position() - N) % (N / 2) == 0 {
                self.sample_rate.div(2);
            }
            self.inner.write(value);
            SamplingOutcome::Consumed
        } else {
            SamplingOutcome::Discarded(value)
        }
    }
}
