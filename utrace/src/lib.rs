#![cfg_attr(not(feature = "std"), no_std)]
#![doc = include_str!("../../README.md")]
use encoding::TracePoint;
pub use utrace_macros::{default_transport, timestamp, trace, trace_here};

pub mod encoding;
mod globals;

#[cfg(not(feature = "std"))]
pub mod tracer;
#[cfg(not(feature = "std"))]
pub use tracer::Tracer;

pub fn init() {
    let _ = crate::globals::default_timestamp_delta();
    encoding::encode(
        TracePoint { delta_t: 0, id: 0 },
        crate::globals::default_write,
    );
}
