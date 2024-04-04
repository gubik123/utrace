#![cfg_attr(not(feature = "std"), no_std)]

pub mod encoding;
mod globals;

#[cfg(not(feature = "std"))]
pub mod tracer;
#[cfg(not(feature = "std"))]
pub use tracer::Tracer;
