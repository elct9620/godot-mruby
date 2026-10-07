//! What each extension keeps for a realm, one value of each type. Values are
//! only ever added, each boxed where it stays, so a reference handed out
//! lives as long as the list does while later values are added.

use std::any::{Any, TypeId};
use std::cell::RefCell;

/// One value of each type an extension asked for, made the first time.
/// Nothing is replaced or removed, which is what lets `data_by_type` lend
/// a value past the list's own borrow.
#[derive(Default)]
pub(super) struct ExtensionData(RefCell<Vec<(TypeId, Box<dyn Any + Send>)>>);

impl ExtensionData {
    /// The value of type `T`, made with its default the first time it is
    /// asked for. The default is made with the list unborrowed, so it may
    /// ask for other data itself.
    pub fn data_by_type<T: Default + Send + 'static>(&self) -> &T {
        if let Some(kept) = self.find::<T>() {
            return kept;
        }
        let made = Box::new(T::default());
        let mut data = self.0.borrow_mut();
        let id = TypeId::of::<T>();
        if !data.iter().any(|(kept, _)| *kept == id) {
            data.push((id, made));
        }
        drop(data);
        self.find().expect("a type's data is kept once made")
    }

    // The value of type `T`, if one is kept. A handful of types are kept, so
    // comparing each one's id beats hashing it.
    fn find<T: 'static>(&self) -> Option<&T> {
        let data = self.0.borrow();
        let id = TypeId::of::<T>();
        let (_, value) = data.iter().find(|(kept, _)| *kept == id)?;
        let found: *const T = value
            .downcast_ref::<T>()
            .expect("a type's data is kept under its own type");
        // SAFETY: this type offers no way to replace or remove a value, and
        // a Box keeps what it holds in place as the list grows, so the value
        // lives as long as `self`.
        Some(unsafe { &*found })
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;

    use super::*;

    #[derive(Default)]
    struct First(Cell<u32>);

    #[derive(Default)]
    struct Second(u64);

    #[test]
    fn a_value_lent_earlier_stays_as_it_was_while_others_are_added() {
        let data = ExtensionData::default();
        let first = data.data_by_type::<First>();
        first.0.set(7);

        let second = data.data_by_type::<Second>();

        assert_eq!(first.0.get(), 7);
        assert_eq!(second.0, 0);
        assert!(std::ptr::eq(first, data.data_by_type::<First>()));
    }

    thread_local! {
        static NESTED_DATA: ExtensionData = ExtensionData::default();
    }

    // Data whose default asks for other data of the same list.
    struct Asking;

    impl Default for Asking {
        fn default() -> Self {
            NESTED_DATA.with(|data| data.data_by_type::<First>().0.set(3));
            Self
        }
    }

    #[test]
    fn a_default_may_ask_for_other_data_as_it_is_made() {
        NESTED_DATA.with(|data| {
            data.data_by_type::<Asking>();

            assert_eq!(data.data_by_type::<First>().0.get(), 3);
        });
    }
}
