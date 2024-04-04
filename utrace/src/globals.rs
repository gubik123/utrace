pub(crate) fn default_write(buf: &[u8]) {
    extern "Rust" {
        fn __utrace_default_transport_write(buf: &[u8]);
    }

    unsafe {
        __utrace_default_transport_write(buf);
    }
}

pub(crate) fn timestamp_delta() -> u32 {
    extern "Rust" {
        fn __utrace_timestamp_function() -> u64;
    }

    static LAST_TIMESTAMP: u64 = 0;

    let current_timestamp = unsafe { __utrace_timestamp_function() };

    0
}
