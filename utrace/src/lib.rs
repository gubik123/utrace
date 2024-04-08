#![cfg_attr(not(feature = "std"), no_std)]

pub use utrace_macros::{default_transport, timestamp, trace, trace_here};

pub mod encoding;
mod globals;

#[cfg(not(feature = "std"))]
pub mod tracer;
#[cfg(not(feature = "std"))]
pub use tracer::Tracer;
