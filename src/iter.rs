/// # Index iterator implementing the infinity sampler algorithm
///
/// This struct implements the [Iterator] trait and generates an infinite sequence of [usize] insertion indexes for the reservoir. This struct's size is fixed and doesn't depend on N.
///
/// This iterator implements the mathematical core of the infinity sampler. It generates an infinite sequence of insertion indexes from 0 to N-1 for a buffer of size N such that:
/// * First N indexes are `0..=N-1`
/// * The rest of the indexes are generated in a loop of size `N*2`
/// * The older the value, the exponentially less likely it is to be kept
///
/// This works together with the dynamic sampling rate implemented by [SamplingReservoir](crate::SamplingReservoir):
///
/// * The sampling rate is halved every time `N/2` (a _pattern_) values have been positively sampled.
/// * At all times, if the reservoir has consumed `M` values, the buffer will contain an even spread of samples, with the distance between samples being exactly either `floor(log2(M))`` or `ceil(log2(M))`.
/// * Each time exactly `N*(2^M)` values consumed by the Reservoir (i.e. `N + M * N/2` values have been positively sampled), the buffer will contain perfectly evenly spead values with indexes `i*(2^M)`.
///
/// ## Example for N=16
///
/// Consider the following chart showing the insertion indexes for a 16-element buffer. The numbers
/// denote value indexes as seen by the Reservoir and only the values selected by the sampler are shown. The time moves from left to right, top to bottom.
///
/// ```ignore
///     |----------------   buffer   -----------------|
///     0  1  2  3  4  5  6  7  8  9  10 11 12 13 14 15     // pattern 0 (initial loop)
///  -     16    18    20    22    24    26    28    30     // pattern 1 / group 0
///  |        32          36          40          44        // pattern 2 / group 0
///  |           48          52          56          60                  / group 1
///  |              64                      72              // pattern 3 / group 0
/// r|                 80                      88                        / group 1
/// e|                    96                      104                    / group 2
/// p|                       112                     120                 / group 3
/// e|                          128                         // pattern 4 / group 0
/// a|                             144                                   / group 1
/// t|                                160                                / group 2
/// s|                                   176                             / group 3
///  |                                      192                          / group 4
///  |                                         208                       / group 5
///  |                                            224                    / group 6
///  -                                               240                 / group 7
/// ```
///
/// ### Item insertion indexes
///
/// ```ignore
/// Pattern 0: 0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15
/// Pattern 1: 1 3 5 7 9 11 13 15
/// Pattern 2: 2 6 10 14
///            3 7 11 15
/// Pattern 3: 4 12
///            5 13
///            6 14
///            7 15
/// Pattenr 4: 8
///            9
///            10
///            11
///            12
///            13
///            14
///            15
/// ```
///
/// ### Buffer contents
///
/// * After 16 items seen: `0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15`
/// * After 32 items seen: `0 2 4 6 8 10 12 14 16 18 20 22 24 26 28 30`
/// * After 64 items seen: `0 4 8 12 16 20 24 28 32 36 40 44 48 52 56 60`
///
#[derive(Clone)]
pub struct InfinitySamplerIndexer<const N: usize> {
    iterator_pos: usize,
    idx: usize,
    left_offset: usize,
    step: usize,
}

impl<const N: usize> InfinitySamplerIndexer<N> {
    /// Create a new iterator for a buffer of size N, starting at 0.
    pub const fn new() -> Self {
        assert!(
            N.is_power_of_two(),
            "Buffer capacity must be a power of two"
        );
        Self {
            iterator_pos: 0,
            idx: 1,
            left_offset: 1,
            step: 2,
        }
    }

    /// Returns the current position of the iterator in terms of total item count.
    /// Increases monotonically.
    pub fn position(&self) -> usize {
        self.iterator_pos
    }

    /// Turn this iterator around, producing a [ReverseInfinitySamplerIndexer].
    /// The next call to [`ReverseInfinitySamplerIndexer::next`] will return the same value as the last call to [`InfinitySamplerIndexer::next`], and then continue backwards.
    pub fn reverse(self) -> ReverseInfinitySamplerIndexer<N> {
        ReverseInfinitySamplerIndexer {
            iterator_pos: self.iterator_pos,
            idx: self.idx,
            left_offset: self.left_offset,
            step: self.step,
        }
    }
}

impl<const N: usize> Default for InfinitySamplerIndexer<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> Iterator for InfinitySamplerIndexer<N> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        let idx = self.iterator_pos;
        self.iterator_pos += 1;

        if idx < N {
            return Some(idx);
        }

        let idx = self.idx;

        if self.idx == N - 1 {
            // end of a pattern
            if self.left_offset == N - 1 {
                // restart the loop
                self.left_offset = 1;
                self.step = 2;
            } else {
                // move on to the next pattern
                self.left_offset += 1;
                debug_assert!(self.left_offset < N);
                self.step *= 2;
            }
            self.idx = self.left_offset;
        } else if self.idx + self.step >= N {
            // move to the next group in the same pattern
            self.left_offset += 1;
            self.idx = self.left_offset;
        } else {
            self.idx += self.step;
        }

        Some(idx)
    }
}

/// [InfinitySamplerIndexer], but in the reverse direction.
///
/// Unlike [InfinitySamplerIndexer], this iterator will end after reaching 0.
#[derive(Clone)]
pub struct ReverseInfinitySamplerIndexer<const N: usize> {
    iterator_pos: usize,
    idx: usize,
    left_offset: usize,
    step: usize,
}

impl<const N: usize> ReverseInfinitySamplerIndexer<N> {
    pub fn position(&self) -> usize {
        self.iterator_pos
    }

    /// Turn this iterator around, producing a [InfinitySamplerIndexer].
    /// The next call to [`InfinitySamplerIndexer::next`] will return the same value as the last call to [`ReverseInfinitySamplerIndexer::next`], and then continue backwards.
    pub fn reverse(self) -> InfinitySamplerIndexer<N> {
        InfinitySamplerIndexer {
            iterator_pos: self.iterator_pos,
            idx: self.idx,
            left_offset: self.left_offset,
            step: self.step,
        }
    }
}

impl<const N: usize> Iterator for ReverseInfinitySamplerIndexer<N> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        if self.iterator_pos == 0 {
            return None;
        }

        self.iterator_pos -= 1;
        if self.iterator_pos < N {
            return Some(self.iterator_pos);
        }

        if self.idx == 1 {
            // end of a loop
            self.left_offset = N - 1;
            self.step = N;
            self.idx = N - 1;
        } else if self.idx == self.left_offset {
            if self.left_offset == self.step / 2 {
                // end of a pattern
                self.step /= 2;
                self.left_offset -= 1;
            } else {
                // end of a group
                self.left_offset -= 1;
            }
            self.idx = self.left_offset + N - self.step;
        } else {
            self.idx -= self.step;
        }

        Some(self.idx)
    }
}
