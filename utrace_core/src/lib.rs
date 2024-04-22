#![cfg_attr(not(feature = "std"), no_std)]

pub mod encoding;

#[cfg(feature = "std")]
pub mod trace_point;

pub const MAX_TRACE_POINTS: usize = 255;
pub const TRACE_POINT_SECTION_NAME: &str = ".utrace_trace_points";
