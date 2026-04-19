//! Utils for interfacing with Embassy's [`PriorityChannel`](embassy_sync::priority_channel::PriorityChannel).

/// Retrieves the numeric discriminant value of an enum.
///
/// See also [`core::mem::discriminant`].
///
/// # Discriminant Value
/// The topmost field's discriminant starts at 0 and increases further down,
/// unless assigned manually.
///
/// # Safety
/// This trait must only be implemented on enums that are
/// marked with `#[repr(u8)]`.
pub(crate) unsafe trait Discriminant {
    #[inline(always)]
    /// Get the numeric discriminant value.
    fn discriminant(&self) -> u8 {
        // SAFETY: Because `Self` is marked `repr(u8)`, its layout is a `repr(C)` `union`
        // between `repr(C)` structs, each of which has the `u8` discriminant as its first
        // field, so we can read the discriminant without offsetting the pointer.
        unsafe { *<*const _>::from(self).cast() }
    }
}

#[repr(transparent)]
/// A wrapper that implements `Eq` and `Ord` traits for an enum `T` without
/// needing `T` itself to implement those traits.
///
/// `Eq` and `Ord` is determined by the discriminant value of the enum.
///
/// This wrapper is made primarily for use in [`PriorityChannel`](embassy_sync::priority_channel::PriorityChannel).
pub(crate) struct Priority<T: Discriminant>(pub T);

impl<T: Discriminant> Priority<T> {
    #[inline(always)]
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T: Discriminant> From<T> for Priority<T> {
    #[inline(always)]
    fn from(value: T) -> Self {
        Self(value)
    }
}

impl<T: Discriminant> PartialEq for Priority<T> {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self.0.discriminant() == other.0.discriminant()
    }
}

impl<T: Discriminant> Eq for Priority<T> {}

impl<T: Discriminant> PartialOrd for Priority<T> {
    #[inline(always)]
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: Discriminant> Ord for Priority<T> {
    #[inline(always)]
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.0.discriminant().cmp(&other.0.discriminant())
    }
}
