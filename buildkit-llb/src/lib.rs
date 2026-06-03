#![deny(warnings)]
#![deny(clippy::all)]

// FIXME: get rid of the unwraps
// TODO: implement warnings for op hash collisions (will incredibly help to debug problems).
// TODO: implement efficient `std::fmt::Debug` for the ops (naive implementation can't handle huge nested graphs).

mod serialization;

/// Supported operations - building blocks of the LLB definition graph.
pub mod ops;

/// Various helpers and types.
pub mod utils;

/// Convenient re-export of a commonly used things.
pub mod prelude {
    pub use crate::ops::exec::Mount;
    pub use crate::ops::fs::LayerPath;
    pub use crate::ops::platform::{self, Platform};
    pub use crate::ops::source::ResolveMode;
    pub use crate::ops::*;
    pub use crate::utils::{OperationOutput, OutputIdx, OwnOutputIdx};
}

/// Re-export of the BuildKit protobuf types so callers can construct the
/// option structs (`ChownOpt`, `CacheOpt`, `SecretOpt`, ...) accepted by the
/// operation builders without depending on `buildkit-proto` directly.
pub use buildkit_proto::pb;
