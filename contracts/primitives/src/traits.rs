//! Define utility traits.

use core::ops::{
    Add, AddAssign, Div, DivAssign, Mul, MulAssign, Rem, RemAssign, Shl, Shr, Sub, SubAssign,
};
pub use num_traits::{
    Bounded, CheckedAdd, CheckedDiv, CheckedMul, CheckedShl, CheckedShr, CheckedSub, One,
    Saturating, Zero,
};

/// Simple trait similar to `Into`, except that it can be used to convert numerics between each
/// other.
pub trait As<T> {
    /// Convert forward (ala `Into::into`).
    fn as_(self) -> T;
    /// Convert backward (ala `From::from`).
    fn sa(_: T) -> Self;
}

macro_rules! impl_numerics {
    ( $( $t:ty ),* ) => {
        $(
            impl_numerics!($t: u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize,);
        )*
    };
    ( $f:ty : $t:ty, $( $rest:ty, )* ) => {
        impl As<$t> for $f {
            fn as_(self) -> $t { self as $t }
            fn sa(t: $t) -> Self { t as Self }
        }
        impl_numerics!($f: $( $rest, )*);
    };
    ( $f:ty : ) => {}
}

impl_numerics!(u8, u16, u32, u64, u128, usize, i8, i16, i32, i64, i128, isize);

/// A type that can be used in runtime structures.
pub trait Member: Send + Sync + Sized + Eq + PartialEq + Clone + 'static {}
impl<T: Send + Sync + Sized + Eq + PartialEq + Clone + 'static> Member for T {}

/// A meta trait for arithmetic.
pub trait SimpleArithmetic:
    Zero
    + One
    + As<u64>
    + As<u128>
    + Add<Self, Output = Self>
    + AddAssign<Self>
    + Sub<Self, Output = Self>
    + SubAssign<Self>
    + Mul<Self, Output = Self>
    + MulAssign<Self>
    + Div<Self, Output = Self>
    + DivAssign<Self>
    + Rem<Self, Output = Self>
    + RemAssign<Self>
    + Shl<u32, Output = Self>
    + Shr<u32, Output = Self>
    + CheckedShl
    + CheckedShr
    + CheckedAdd
    + CheckedSub
    + CheckedMul
    + CheckedDiv
    + Saturating
    + PartialOrd<Self>
    + Ord
    + Bounded
{
}
impl<
        T: Zero
            + One
            + As<u64>
            + As<u128>
            + Add<Self, Output = Self>
            + AddAssign<Self>
            + Sub<Self, Output = Self>
            + SubAssign<Self>
            + Mul<Self, Output = Self>
            + MulAssign<Self>
            + Div<Self, Output = Self>
            + DivAssign<Self>
            + Rem<Self, Output = Self>
            + RemAssign<Self>
            + Shl<u32, Output = Self>
            + Shr<u32, Output = Self>
            + CheckedShl
            + CheckedShr
            + CheckedAdd
            + CheckedSub
            + CheckedMul
            + CheckedDiv
            + Saturating
            + PartialOrd<Self>
            + Ord
            + Bounded,
    > SimpleArithmetic for T
{
}
