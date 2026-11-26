//! Unique identifiers for document elements.
//!
//! A single generic [`Id<T>`] backs every kind of identifier; the phantom
//! marker `T` keeps page, layer, object, master-page, and style IDs from being
//! used interchangeably. IDs are process-unique via a monotonic counter.

use std::marker::PhantomData;
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_ID: AtomicU64 = AtomicU64::new(1);

fn next_id() -> u64 {
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}

/// Record an externally supplied ID (from a loaded file or raw value) so the
/// generator never hands it out again. Without this, objects created after a
/// document is opened could reuse IDs already present in it.
fn observe(id: u64) {
    NEXT_ID.fetch_max(id.saturating_add(1), Ordering::Relaxed);
}

/// Reset the ID counter. Intended for tests and deterministic-ID scenarios
/// such as import-stability checks; misuse in normal code risks collisions.
pub fn reset_id_counter(value: u64) {
    NEXT_ID.store(value, Ordering::Relaxed);
}

/// A process-unique identifier tagged by marker type `T`.
pub struct Id<T> {
    value: u64,
    _marker: PhantomData<fn() -> T>,
}

impl<T> Id<T> {
    /// Allocate a fresh unique identifier.
    pub fn new() -> Self {
        Self::from_value(next_id())
    }

    /// Wrap a raw value coming from outside the generator, recording it so it
    /// is never allocated again.
    pub fn from_raw(value: u64) -> Self {
        observe(value);
        Self::from_value(value)
    }

    /// The underlying numeric value.
    pub fn raw(&self) -> u64 {
        self.value
    }

    fn from_value(value: u64) -> Self {
        Self {
            value,
            _marker: PhantomData,
        }
    }
}

impl<T> Default for Id<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Clone for Id<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for Id<T> {}

impl<T> PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<T> Eq for Id<T> {}

impl<T> std::hash::Hash for Id<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.value.hash(state);
    }
}

impl<T: IdKind> std::fmt::Debug for Id<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}_{}", T::PREFIX, self.value)
    }
}

impl<T: IdKind> std::fmt::Display for Id<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}_{}", T::PREFIX, self.value)
    }
}

/// Marker trait implemented by each ID kind, supplying its display prefix.
pub trait IdKind {
    /// Short prefix used when displaying the ID (e.g. `"obj"`).
    const PREFIX: &'static str;
}

/// Declare an ID marker type and its type alias.
macro_rules! id_kind {
    ($(#[$meta:meta])* $marker:ident => $alias:ident, $prefix:literal) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum $marker {}

        impl IdKind for $marker {
            const PREFIX: &'static str = $prefix;
        }

        #[doc = concat!("A unique identifier for a ", $prefix, ".")]
        pub type $alias = Id<$marker>;
    };
}

id_kind!(/// Marker for page IDs.
    PageMarker => PageId, "page");
id_kind!(/// Marker for layer IDs.
    LayerMarker => LayerId, "layer");
id_kind!(/// Marker for object IDs.
    ObjectMarker => ObjectId, "obj");
id_kind!(/// Marker for master-page IDs.
    MasterPageMarker => MasterPageId, "master");
id_kind!(/// Marker for style IDs.
    StyleMarker => StyleId, "style");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique() {
        assert_ne!(ObjectId::new(), ObjectId::new());
    }

    #[test]
    fn display_uses_prefix() {
        assert_eq!(format!("{}", ObjectId::from_raw(42)), "obj_42");
        assert_eq!(format!("{}", PageId::from_raw(7)), "page_7");
    }

    #[test]
    fn loaded_ids_are_not_reused() {
        reset_id_counter(1);
        let loaded = ObjectId::from_raw(1_000_000);
        let fresh = ObjectId::new();
        assert!(fresh.raw() > loaded.raw());
    }

    #[test]
    fn distinct_kinds_do_not_collide_in_maps() {
        // Compile-time check: these are different types.
        let _p: PageId = PageId::new();
        let _o: ObjectId = ObjectId::new();
    }
}
