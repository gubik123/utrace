use utrace_core::encoding::encode;
use utrace_core::encoding::TracePoint;

pub struct Tracer {
    exit_id: Option<u8>,
}

pub enum SkipConfig {
    NoSkip,
    Skip {
        counter: &'static mut u32,
        limit: u32,
    },
}

impl Tracer {
    pub fn new(entry_id: Option<u8>, exit_id: Option<u8>, skip_config: SkipConfig) -> Option<Self> {
        match skip_config {
            SkipConfig::NoSkip => {
                if let Some(id) = entry_id {
                    Self::emit(id);
                }
                Some(Tracer { exit_id })
            }
            SkipConfig::Skip { counter, limit } => {
                *counter += 1;
                if *counter >= limit {
                    if let Some(id) = entry_id {
                        Self::emit(id);
                    }
                    *counter = 0;
                    Some(Tracer { exit_id })
                } else {
                    None
                }
            }
        }
    }

    fn emit(id: u8) {
        critical_section::with(|_| {
            let delta = crate::globals::default_timestamp_delta();
            encode(
                TracePoint { delta_t: delta, id },
                crate::globals::default_write,
            );
        });
    }
}

impl Drop for Tracer {
    fn drop(&mut self) {
        if let Some(id) = self.exit_id {
            Self::emit(id);
        }
    }
}

// Additional Implementations and Debug Trait for better logging
impl core::fmt::Debug for Tracer {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Tracer")
            .field("exit_id", &self.exit_id)
            .finish()
    }
}

impl core::fmt::Debug for SkipConfig {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            SkipConfig::NoSkip => write!(f, "NoSkip"),
            SkipConfig::Skip { counter, limit } => f
                .debug_struct("Skip")
                .field("counter", counter)
                .field("limit", limit)
                .finish(),
        }
    }
}
