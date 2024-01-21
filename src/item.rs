use core::mem::MaybeUninit;
use core::num::NonZeroUsize;
use core::ops::{Deref, DerefMut};

/// A single item in the reservoir's buffer.
/// This is a thin wrapper over [MaybeUninit] that also tracks the insertion index
/// and initialization state.
pub struct Item<T> {
    pub(crate) insertion_index: Option<NonZeroUsize>,
    value: MaybeUninit<T>,
}

/// A single [Item] that is guaranteed to be initialized.
///
/// This struct has identical layout to [Item] and is actually just a cast.
/// Implements [Deref] and [DerefMut] towards `T`.
pub struct InitializedItem<T> {
    _pad: Option<NonZeroUsize>,
    pub(crate) value: T,
}

impl<T> Item<T> {
    pub(crate) const fn empty() -> Self {
        Self {
            insertion_index: None,
            value: MaybeUninit::uninit(),
        }
    }

    /// Take the value out of an item and reset its state.
    #[must_use]
    pub fn take(&mut self) -> Option<T> {
        if let Some(_) = self.insertion_index.take() {
            // SAFETY: the value is initialized
            Some(unsafe { self.value.assume_init_read() })
        } else {
            None
        }
    }

    /// Take the value out of an item without checking its state.
    /// UB if the item is empty.
    pub unsafe fn take_unchecked(&mut self) -> T {
        debug_assert!(self.insertion_index.is_some());
        // SAFETY: the value is initialized
        self.insertion_index.take();
        unsafe { self.value.assume_init_read() }
    }

    /// Write a new value into an item, returning the old value
    pub fn write(&mut self, index: NonZeroUsize, value: T) -> Option<(NonZeroUsize, T)> {
        let old_value = match self.insertion_index {
            Some(index) => {
                // SAFETY: the value is initialized
                Some((index, unsafe {
                    core::mem::replace(&mut self.value, MaybeUninit::uninit()).assume_init()
                }))
            }
            None => None,
        };
        self.insertion_index = Some(index);
        self.value.write(value);
        old_value
    }

    /// Assume the item is not empty and return a reference.
    /// UB if the item is empty.
    pub const unsafe fn assume_init_ref(&self) -> &T {
        debug_assert!(self.insertion_index.is_some());
        // SAFETY: the value is initialized
        unsafe { self.value.assume_init_ref() }
    }
}

impl<T> Drop for Item<T> {
    fn drop(&mut self) {
        if self.insertion_index.is_some() {
            // SAFETY: the value is initialized
            unsafe { self.value.assume_init_drop() };
        }
    }
}

impl<T: Clone> Clone for Item<T> {
    fn clone(&self) -> Self {
        match self.insertion_index {
            Some(_) => Self {
                insertion_index: self.insertion_index,
                value: MaybeUninit::new(unsafe { self.value.assume_init_ref().clone() }),
            },
            None => Self {
                insertion_index: None,
                value: MaybeUninit::uninit(),
            },
        }
    }
}

impl<T> Deref for InitializedItem<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl DerefMut for InitializedItem<u32> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}
