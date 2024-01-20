//! # Algorithm explainer
//!
//! The insertion indexes are from _0_ to _N-1_ for a buffer of size _N_ such that:
//! * The first _N_ indexes are _0..=N-1_
//! * The rest of the indexes are generated in a loop of size _2N_
//! * The older the value, the exponentially less likely it is to be kept
//!
//! The algorithm is as follows:
//!
//! * Consider a buffer of size _N_ and a stream of values _V<sub>i</sub>_.
//! * Store the value _V<sub>0</sub>_ at index _0_.
//! * For _V<sub>i</sub>_, first make a sampling decision:
//!     * Consider the significant bits of the binary representation of _i_.
//!     * Remove _log<sub>2</sub>N_ most significant bits.
//!     * If the remainder is not zero, discard _V<sub>i</sub>_.
//! * Store _V<sub>i</sub>_ at index _(i - 1) mod (N - 1) + 1_.
//!
//! This has following properties:
//!
//! * The sampling rate is halved every time another _N/2_ values have been selected.
//! * At all times, if the reservoir has observed _M_ values, the buffer will contain an even spread of samples with the distance between samples being exactly either 2 <sup>⌊log<sub>2</sub>(M/N)⌋</sup> or 2<sup>⌈log<sub>2</sub>(M/N)⌉</sup> i.e. nearest powers of 2 to _M/N_.
//! * Each time exactly _N * 2<sup>M</sup>_ values observed by the Reservoir (i.e. _N + MN/2_ values have been positively sampled), the buffer will contain perfectly evenly spead values with indexes _2<sup>M</sup>x_.
//!
//! ## Example for N=16
//!
//! Consider the following chart showing the insertion indexes for a 16-element buffer. The numbers
//! denote value indexes as seen by the Reservoir and only the values selected by the sampler are shown. The time moves from left to right, top to bottom.
//!
//! ```ignore
//!     |----------------   buffer   -----------------|
//!     0  1  2  3  4  5  6  7  8  9  10 11 12 13 14 15     // pattern 0 (initial loop)
//!  -     16    18    20    22    24    26    28    30     // pattern 1 / group 0
//!  |        32          36          40          44        // pattern 2 / group 0
//!  |           48          52          56          60                  / group 1
//!  |              64                      72              // pattern 3 / group 0
//! r|                 80                      88                        / group 1
//! e|                    96                      104                    / group 2
//! p|                       112                     120                 / group 3
//! e|                          128                         // pattern 4 / group 0
//! a|                             144                                   / group 1
//! t|                                160                                / group 2
//! s|                                   176                             / group 3
//!  |                                      192                          / group 4
//!  |                                         208                       / group 5
//!  |                                            224                    / group 6
//!  -                                               240                 / group 7
//! ```
//!
//! ### Item insertion indices
//!
//! ```ignore
//! Pattern 0: 0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15
//! Pattern 1: 1 3 5 7 9 11 13 15
//! Pattern 2: 2 6 10 14
//!            3 7 11 15
//! Pattern 3: 4 12
//!            5 13
//!            6 14
//!            7 15
//! Pattenr 4: 8
//!            9
//!            10
//!            11
//!            12
//!            13
//!            14
//!            15
//! ```
//!
//! ### Buffer contents (sorted)
//!
//! * After 16 items observed: `0 1 2 3 4 5 6 7 8 9 10 11 12 13 14 15`
//! * After 32 items observed: `0 2 4 6 8 10 12 14 16 18 20 22 24 26 28 30`
//! * After 64 items observed: `0 4 8 12 16 20 24 28 32 36 40 44 48 52 56 60`
//!
