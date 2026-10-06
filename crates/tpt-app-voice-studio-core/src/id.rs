//! Strongly typed identifiers for the Voice Studio domain model.
//!
//! Every entity id is a distinct newtype over `u64` so ids cannot be mixed up
//! across entity kinds (a [`SegmentId`] is not a [`SpeakerId`]), while staying
//! cheap to copy, hash, and persist.

use std::fmt;

macro_rules! define_id {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default,
            serde::Serialize, serde::Deserialize)]
        #[serde(transparent)]
        pub struct $name(u64);

        impl $name {
            /// Creates an id from the raw numeric value.
            #[must_use]
            pub const fn new(value: u64) -> Self {
                Self(value)
            }

            /// Returns the raw numeric value.
            #[must_use]
            pub const fn get(self) -> u64 {
                self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, concat!(stringify!($name), "({})"), self.0)
            }
        }
    };
}

define_id! {
    /// Identifier of a [`crate::id`] scoped project (spec §6.1).
    ProjectId
}
define_id! {
    /// Identifier of a source recording in a project (spec §6.2).
    RecordingId
}
define_id! {
    /// Identifier of a transcript segment (spec §6.3).
    SegmentId
}
define_id! {
    /// Identifier of a speaker in a project's speaker registry (spec §6.4).
    SpeakerId
}
define_id! {
    /// Identifier of an export job (spec §6.6).
    ExportJobId
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_of_different_types_are_distinct_types() {
        let segment = SegmentId::new(1);
        let speaker = SpeakerId::new(1);
        // Both wrap the same value but cannot be compared or exchanged.
        assert_eq!(segment.get(), speaker.get());
        assert_ne!(segment, SegmentId::new(2));
    }

    #[test]
    fn id_display_includes_type_name() {
        assert_eq!(format!("{}", RecordingId::new(7)), "RecordingId(7)");
    }

    #[test]
    fn ids_sort_by_raw_value() {
        let mut ids = [SegmentId::new(9), SegmentId::new(1), SegmentId::new(4)];
        ids.sort();
        assert_eq!(
            ids,
            [SegmentId::new(1), SegmentId::new(4), SegmentId::new(9)]
        );
    }
}
