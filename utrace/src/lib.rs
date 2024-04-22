#![no_std]
#![doc = include_str!("../../README.md")]
use utrace_core::encoding::TracePoint;
pub use utrace_macros::{default_transport, timestamp, trace, trace_here};

mod globals;

pub mod tracer;
pub use tracer::Tracer;

pub fn init() {
    let _ = crate::globals::default_timestamp_delta();
    utrace_core::encoding::encode(
        TracePoint { delta_t: 0, id: 0 },
        crate::globals::default_write,
    );
}
