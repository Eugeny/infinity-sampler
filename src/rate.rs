/// A simple sampler.
///
/// ```
/// use infinity_sampler::SamplingRate;
///
/// let mut sampler = SamplingRate::new(2);
/// assert_eq!(sampler.step(), false);
/// assert_eq!(sampler.step(), true);
/// assert_eq!(sampler.step(), false);
/// assert_eq!(sampler.step(), true);
///
/// sampler.div(2);
/// assert_eq!(sampler.step(), false);
/// assert_eq!(sampler.step(), false);
/// assert_eq!(sampler.step(), false);
/// assert_eq!(sampler.step(), true);
/// assert_eq!(sampler.step(), false);
/// assert_eq!(sampler.step(), false);
/// assert_eq!(sampler.step(), false);
/// assert_eq!(sampler.step(), true);
///
#[derive(Copy, Clone)]
pub struct SamplingRate {
    divisor: u32,
    counter: u32,
}

impl SamplingRate {
    pub const fn new(divisor: u32) -> Self {
        Self {
            divisor,
            counter: 0,
        }
    }

    /// Returns true if the sampler should sample.
    pub fn step(&mut self) -> bool {
        self.counter += 1;
        self.counter %= self.divisor;
        self.counter == 0
    }

    /// Reduce the sampling rate by a ratio.
    pub fn div(&mut self, ratio: u32) {
        self.divisor *= ratio;
    }
}
