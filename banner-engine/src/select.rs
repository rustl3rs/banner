/// The `Any` type. Analogous to `Option` but representing the possibility of
/// any type at all instead of no type.
#[derive(Clone, Copy, PartialOrd, Eq, Ord, PartialEq, Debug, Hash)]
pub enum Select<T> {
    /// Any value.
    Any,
    /// Only a specific value of type `T`.
    Only(T),
}

impl<T> Select<T> {
    pub const fn is_specific(&self) -> bool {
        matches!(*self, Select::Only(_))
    }

    // #[must_use]
    // #[inline]
    // pub const fn is_specific_and(self, f: impl FnOnce(T) -> bool) -> bool {
    //     match self {
    //         Any::Any => false,
    //         Any::Only(x) => f(x),
    //     }
    // }

    pub const fn is_any(&self) -> bool {
        !self.is_specific()
    }

    pub fn unwrap(self) -> T {
        match self {
            Select::Only(val) => val,
            Select::Any => panic!("called `Any::unwrap()` on a `Any` value"),
        }
    }
}
