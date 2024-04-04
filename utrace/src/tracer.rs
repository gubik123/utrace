#![no_std]
use defmt;
pub use utrace_macros::{trace, trace_here};

pub struct Tracer {
    exit_id: Option<u8>,
}

impl Tracer {
    pub fn new(entry_id: Option<u8>, exit_id: Option<u8>) -> Tracer {
        if let Some(id) = entry_id {
            defmt::info!("Hello, world from ID = {}", id);
        }

        Tracer { exit_id }
    }
}

impl Drop for Tracer {
    fn drop(&mut self) {
        if let Some(id) = self.exit_id {
            defmt::info!("Hello, world from ID = {}", id);
        }
    }
}
