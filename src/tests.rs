use std::vec::Vec;

use crate::*;

#[test]
#[should_panic]
fn size_check() {
    SamplingReservoir::<u32, 12>::new();
}

#[test]
fn storage_idx_n_8() {
    let indices = (0..256)
        .into_iter()
        .filter(|x| SamplingReservoir::<u32, 8>::should_sample(*x))
        .map(|x| SamplingReservoir::<u32, 8>::storage_index_for_outer_index(x))
        .collect::<Vec<_>>();

    assert_eq!(
        &indices[..],
        &[0, 1, 2, 3, 4, 5, 6, 7, 1, 3, 5, 7, 2, 6, 3, 7, 4, 5, 6, 7, 1, 3, 5, 7, 2, 6, 3, 7]
    );
}

#[test]
fn storage_idx_n_16() {
    let indices = (0..256)
        .into_iter()
        .filter(|x| SamplingReservoir::<u32, 16>::should_sample(*x))
        .map(|x| SamplingReservoir::<u32, 16>::storage_index_for_outer_index(x))
        .collect::<Vec<_>>();

    assert_eq!(
        &indices[..],
        &[
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 1, 3, 5, 7, 9, 11, 13, 15, 2, 6,
            10, 14, 3, 7, 11, 15, 4, 12, 5, 13, 6, 14, 7, 15, 8, 9, 10, 11, 12, 13, 14, 15
        ]
    );
}

#[test]
fn insertion_sampling_sm() {
    let mut buf = SamplingReservoir::<u32, 8>::new();
    for i in 0..32 {
        buf.sample(i);
    }
    let mut inner = buf
        .as_unordered_slice()
        .into_iter()
        .map(|x| *x)
        .collect::<Vec<_>>();
    inner.sort();
    assert_eq!(&inner[..], &[0, 4, 8, 12, 16, 20, 24, 28]);
}

#[test]
fn insertion_sampling_lg() {
    let mut buf = SamplingReservoir::<u32, 16>::new();
    for i in 0..32 {
        buf.sample(i);
    }
    let mut inner = buf
        .as_unordered_slice()
        .into_iter()
        .map(|x| *x)
        .collect::<Vec<_>>();
    inner.sort();
    assert_eq!(
        &inner[..],
        &[0, 2, 4, 6, 8, 10, 12, 14, 16, 18, 20, 22, 24, 26, 28, 30]
    );

    let mut buf = SamplingReservoir::<u32, 16>::new();
    for i in 0..64 {
        buf.sample(i);
    }
    let mut inner = buf
        .as_unordered_slice()
        .into_iter()
        .map(|x| *x)
        .collect::<Vec<_>>();
    inner.sort();
    assert_eq!(
        &inner[..],
        &[0, 4, 8, 12, 16, 20, 24, 28, 32, 36, 40, 44, 48, 52, 56, 60]
    );

    let mut buf = SamplingReservoir::<u32, 16>::new();
    for i in 0..256 {
        buf.sample(i);
    }
    let mut inner = buf
        .as_unordered_slice()
        .into_iter()
        .map(|x| *x)
        .collect::<Vec<_>>();
    inner.sort();
    assert_eq!(
        &inner[..],
        &[0, 16, 32, 48, 64, 80, 96, 112, 128, 144, 160, 176, 192, 208, 224, 240]
    );
}

#[test]
fn sample_rate() {
    let mut buf = SamplingReservoir::<usize, 16>::new();
    for i in 0..10000 {
        assert_eq!(
            matches!(buf.sample(i), SamplingOutcome::Discarded(_)),
            !SamplingReservoir::<usize, 16>::should_sample(i)
        );
    }
}

#[test]
fn e2e_full() {
    let mut buf = SamplingReservoir::<u32, 8>::new();
    for i in 0..256 {
        buf.sample(i);
    }
    let result = buf.into_ordered_iter().collect::<Vec<_>>();
    assert_eq!(&result[..], &[0, 32, 64, 96, 128, 160, 192, 224]);
}

#[test]
fn e2e_partial_fill() {
    let mut buf = SamplingReservoir::<u32, 8>::new();
    for i in 0..4 {
        buf.sample(i);
    }
    let result = buf.into_ordered_iter().collect::<Vec<_>>();
    assert_eq!(&result[..], &[0, 1, 2, 3]);
}

#[test]
fn e2e_mid_loop() {
    let mut buf = SamplingReservoir::<u32, 16>::new();
    for i in 0..50 {
        buf.sample(i);
    }
    let result = buf.into_ordered_iter().collect::<Vec<_>>();
    assert_eq!(
        &result[..],
        &[0, 4, 8, 12, 16, 20, 22, 24, 26, 28, 30, 32, 36, 40, 44, 48]
    );
}

#[test]
fn e2e_fuzz() {
    for i in 1..100 {
        let mut buf = SamplingReservoir::<u32, 16>::new();
        for j in 0..i {
            buf.sample(j);
        }
        let result = buf.into_ordered_iter().collect::<Vec<_>>();
        let mut sorted = result.iter().copied().collect::<Vec<_>>();
        sorted.sort();
        assert_eq!(result, sorted);
    }
}

#[test]
fn leak_test() {
    // Use vecs to trigger Miri leak detector
    for i in 1..100 {
        let mut buf = SamplingReservoir::<Vec<u8>, 16>::new();
        for _ in 0..i {
            buf.sample(vec![0]);
        }

        let _ = buf.clone().into_ordered_iter().collect::<Vec<_>>();
    }
}
