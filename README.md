# utrace
utrace is an instrumentation-based profiling tool for embedded applications. It is intended to be
platform-agnostic, async-friendly and low-overhead. The principle of operation of utrace was inspired by the fantastic [defmt logging library](https://defmt.ferrous-systems.com/).

## Usage
Main user-facing APIs are the two procedural macros that can be used to insert instrumentation - the attribute
[#\[trace\]](crate::trace) and the function-like [trace_here]. Possible usages are demonstrated in this snippet:

```rust
#[trace]
async fn do_something() {

}

#[trace]
fn do_something_else() {

}

{
    trace_here!();
    ...
    ...
}
```

When [#\[trace\]](crate::trace) instruments an async function, the instants of the respective future creation, dropping and poll spans will be reported by default.

<div class="warning">
The current implementation assumes that traced Futures are not reentrant (it implies that only one instance of an instrumented async function is pending at each moment). If this is not the case, Future lifecycle tracking will be broken. 
</div>

## Trace information timestamping and transport
While tracing instrumentation itself is platform-agnostic, it requires a way of obtaining timestamps and a channel for data transfer from dut to the host system.

To provide a timestamp function to the library, use the [#\[timestamp\]](crate::timestamp) macro. For example:

```rust
#[utrace::timestamp]
fn utrace_timestamp_fn() -> u64 {
    (Tim15::now() - <Tim15 as Monotonic>::ZERO).to_micros()
}
```

In the current version, the signature of the timestamp function must be `rust fn() -> u64`.

To define a transport, annotate a function with [#\[utrace::default_transport\]] like this:

```rust
#[utrace_macros::default_transport]
pub fn write(buf: &[u8]) {
    ...
}
```

The current implementation provides the implementation of RTT-based transport in *utrace_rtt* crate.

Note, that current implementation requires an implementation of a critical section. For example, if you
are using single-core ARM MCU, you can add

```toml
cortex-m = { version = "0.7.7", features = ["critical-section-single-core"] }
```

to your Cargo.toml.

## Trace data interpretation
The metadata, required for trace interpretation is stored in the output elf binary. Correct bundling of this metadata requires passing
*utrace_linker.x* script to a linker during your binary linking. It could be done either in *build.rs* script by adding something like

```rust
println!("cargo::rustc-link-arg=-Tutrace_linker.x");
```

or by adding

```toml
rustflags = [
  "-C", "link-arg=-Tutrace_linker.x"
]
```
to your *.cargo/config.toml*. The first method should be prefered to the second one.

To extract metadata and interpret trace data stream, *utrace_parser* crate should be used. This crate's package provides a binary, called *utrace-capture*, which can receive raw trace stream from TCP connection or stdin and write the trace in *chrome://tracing* format. To install it, execute

```bash
cargo install --locked utrace_parser --features="cli"
```

Let's assume that OpenOCD is used for DUT interface. In this case, you should have something like this in OpenOCD init script:

```tcl
rtt setup 0x20000000 0x20000 "SEGGER RTT"
rtt server start 9001 0
rtt start
```

This will tell OpenOCD to listen on port 9001 and send raw RTT data from channel 0 to a connected client. To capture and save trace stream in this configuration, run

```bash
utrace-capture <path to firmware elf executable> --tcp localhost:9001 --out-ct trace_out
```

Traces will be captured in *trace_out_xxx.json* files. To finish capture, press *Ctrl+C*. These traces can be opened with [chrome://tracing](chrome://tracing) if you are using Chrome browser, or with [Perfetto UI](https://ui.perfetto.dev).

Note, that probe-rs RTT feature tries to connect to server, instead of listening on a port, so you will need to use `--tcp-listen`
flag, eg:

```bash
utrace-capture <path to firmware elf executable> --tcp-listen 0.0.0.0:9001 --out-ct trace_out
```