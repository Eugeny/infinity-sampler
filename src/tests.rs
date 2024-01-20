use std::vec::Vec;

#[test]
#[should_panic]
fn size_check() {
    crate::RawReservoir::<u32, 12>::new();
}

#[test]
fn pattern_idx() {
    let indices = (0..64)
        .into_iter()
        .map(|x| crate::RawReservoir::<u32, 16>::_pattern_index_for_in_loop_index(x))
        .collect::<Vec<_>>();

    assert_eq!(
        &indices[..],
        &[
            1, 1, 1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7, 8, 9, 10, 11,
            12, 13, 14, 15, 1, 1, 1, 1, 1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3, 4, 4, 5, 5, 6, 6, 7, 7,
            8, 9, 10, 11, 12, 13, 14, 15,
        ]
    );
}

#[test]
fn storage_idx() {
    let indices = (0..48)
        .into_iter()
        .map(|x| crate::RawReservoir::<u32, 16>::_optimized_storage_index(x))
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
fn storage_idx_naive() {
    let indices = (0..48)
        .into_iter()
        .map(|x| crate::RawReservoir::<u32, 16>::_naive_storage_index(x))
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
fn storage_idx_naive_vs_stateless() {
    let stateless_indices = (0..1024)
        .into_iter()
        .map(|x| crate::RawReservoir::<u32, 16>::_optimized_storage_index(x))
        .collect::<Vec<_>>();
    let naive_indices = (0..1024)
        .into_iter()
        .map(|x| crate::RawReservoir::<u32, 16>::_naive_storage_index(x))
        .collect::<Vec<_>>();

    assert_eq!(&stateless_indices[..], &naive_indices[..],);
}

#[test]
fn storage_idx_naive_vs_iter() {
    let iter_indices = crate::InfinitySamplerIndexer::<16>::default()
        .take(1024)
        .collect::<Vec<_>>();

    let naive_indices = (0..1024)
        .into_iter()
        .map(|x| crate::RawReservoir::<u32, 16>::_naive_storage_index(x))
        .collect::<Vec<_>>();

    assert_eq!(&iter_indices[..], &naive_indices[..],);
}

#[test]
fn storage_idx_iter() {
    let indices = crate::InfinitySamplerIndexer::<16>::default()
        .take(48)
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
fn storage_idx_iter_rev() {
    // 1 - within the initial loop
    let mut iter = crate::InfinitySamplerIndexer::<16>::default();

    let indices = (0..10)
        .into_iter()
        .map(|_| iter.next().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(&indices[..], &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);

    let iter = iter.reverse();
    let indices = iter.collect::<Vec<_>>();
    assert_eq!(&indices[..], &[9, 8, 7, 6, 5, 4, 3, 2, 1, 0]);

    // 2 - mid 2nd loop
    let mut iter = crate::InfinitySamplerIndexer::<16>::default();

    let indices = (0..20)
        .into_iter()
        .map(|_| iter.next().unwrap())
        .collect::<Vec<_>>();
    let mut expected = [
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 1, 3, 5, 7,
    ];
    assert_eq!(&indices[..], &expected);

    let iter = iter.reverse();
    let indices = iter.collect::<Vec<_>>();
    expected.reverse();
    assert_eq!(&indices[..], &expected);

    // 3 - multiple loops
    let mut iter = crate::InfinitySamplerIndexer::<8>::default();

    let indices = (0..32)
        .into_iter()
        .map(|_| iter.next().unwrap())
        .collect::<Vec<_>>();
    let mut expected = [
        0, 1, 2, 3, 4, 5, 6, 7, 1, 3, 5, 7, 2, 6, 3, 7, 4, 5, 6, 7, 1, 3, 5, 7, 2, 6, 3, 7, 4, 5,
        6, 7,
    ];
    assert_eq!(&indices[..], &expected);

    let iter = iter.reverse();
    let indices = iter.collect::<Vec<_>>();
    expected.reverse();
    assert_eq!(&indices[..], &expected);

    // 4 - mid later loop
    let mut iter = crate::InfinitySamplerIndexer::<16>::default();

    let indices = (0..50)
        .into_iter()
        .map(|_| iter.next().unwrap())
        .collect::<Vec<_>>();
    let mut expected = [
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 1, 3, 5, 7, 9, 11, 13, 15, 2, 6, 10,
        14, 3, 7, 11, 15, 4, 12, 5, 13, 6, 14, 7, 15, 8, 9, 10, 11, 12, 13, 14, 15, 1, 3,
    ];
    assert_eq!(&indices[..], &expected);

    let iter = iter.reverse();
    let indices = iter.collect::<Vec<_>>();
    expected.reverse();
    assert_eq!(&indices[..], &expected);
}

#[test]
fn insertion_unconditional() {
    let mut buf = crate::RawReservoir::<u32, 16>::new();
    for i in 0..24 {
        buf.write(i);
    }
    let inner = buf
        .as_unordered_slice()
        .into_iter()
        .map(|x| **x)
        .collect::<Vec<_>>();
    assert_eq!(
        &inner[..],
        &[0, 16, 2, 17, 4, 18, 6, 19, 8, 20, 10, 21, 12, 22, 14, 23]
    );
}

#[test]
fn insertion_sampling_sm() {
    let mut buf = crate::SamplingReservoir::<u32, 8>::new();
    for i in 0..32 {
        buf.sample(i);
    }
    let mut inner = buf
        .as_unordered_slice()
        .into_iter()
        .map(|x| **x)
        .collect::<Vec<_>>();
    inner.sort();
    assert_eq!(&inner[..], &[0, 4, 8, 12, 16, 20, 24, 28]);
}

#[test]
fn insertion_sampling_lg() {
    let mut buf = crate::SamplingReservoir::<u32, 16>::new();
    for i in 0..32 {
        buf.sample(i);
    }
    let mut inner = buf
        .as_unordered_slice()
        .into_iter()
        .map(|x| **x)
        .collect::<Vec<_>>();
    inner.sort();
    assert_eq!(
        &inner[..],
        &[0, 2, 4, 6, 8, 10, 12, 14, 16, 18, 20, 22, 24, 26, 28, 30]
    );

    let mut buf = crate::SamplingReservoir::<u32, 16>::new();
    for i in 0..64 {
        buf.sample(i);
    }
    let mut inner = buf
        .as_unordered_slice()
        .into_iter()
        .map(|x| **x)
        .collect::<Vec<_>>();
    inner.sort();
    assert_eq!(
        &inner[..],
        &[0, 4, 8, 12, 16, 20, 24, 28, 32, 36, 40, 44, 48, 52, 56, 60]
    );

    let mut buf = crate::SamplingReservoir::<u32, 16>::new();
    for i in 0..256 {
        buf.sample(i);
    }
    let mut inner = buf
        .as_unordered_slice()
        .into_iter()
        .map(|x| **x)
        .collect::<Vec<_>>();
    inner.sort();
    assert_eq!(
        &inner[..],
        &[0, 16, 32, 48, 64, 80, 96, 112, 128, 144, 160, 176, 192, 208, 224, 240]
    );
}

#[test]
fn e2e_full() {
    let mut buf = crate::SamplingReservoir::<u32, 8>::new();
    for i in 0..256 {
        buf.sample(i);
    }
    let result = buf.into_ordered_iter().collect::<Vec<_>>();
    assert_eq!(&result[..], &[0, 32, 64, 96, 128, 160, 192, 224]);
}

#[test]
fn e2e_partial_fill() {
    let mut buf = crate::SamplingReservoir::<u32, 8>::new();
    for i in 0..4 {
        buf.sample(i);
    }
    let result = buf.into_ordered_iter().collect::<Vec<_>>();
    assert_eq!(&result[..], &[0, 1, 2, 3]);
}

#[test]
fn e2e_mid_loop() {
    let mut buf = crate::SamplingReservoir::<u32, 16>::new();
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
fn leak_test() {
    // Use vecs to trigger Miri leak detector

    let mut buf = crate::SamplingReservoir::<Vec<u8>, 8>::new();
    for _ in 0..4 {
        buf.sample(vec![0]);
    }

    let _ = buf.clone().into_ordered_iter().collect::<Vec<_>>();

    let mut buf = crate::SamplingReservoir::<Vec<u8>, 8>::new();
    for _ in 0..16 {
        buf.sample(vec![0]);
    }

    let _ = buf.clone().into_ordered_iter().collect::<Vec<_>>();
}
