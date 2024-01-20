# Infinity Sampler

✨ Allocation free ✨ Minimal dependencies ✨ `no_std` compatible ✨

The Infinity Sampler lets you automatically sample an infinite stream of values into a fixed size buffer, while keeping an even spread of samples.

It's a deterministic variation of the [Reservoir Sampling](https://en.wikipedia.org/wiki/Reservoir_sampling) algorithm. Writes are *O(1)*, iteration is *O(N\*log(N))*. See [InfinitySamplerIndexer] for an illustrated explainer.

Your primary interface is the [SamplingReservoir] struct:

```
use infinity_sampler::SamplingReservoir;

let mut reservoir = SamplingReservoir::<u32, 8>::new();
for i in 0..256 {
   reservoir.sample(i);
}
let samples: Vec<_> = reservoir.into_ordered_iter().collect();

assert_eq!(samples, vec![0, 32, 64, 96, 128, 160, 192, 224]);
```

Both reservoir types require `N * (sizeof<T> + sizeof<usize>)` memory.
