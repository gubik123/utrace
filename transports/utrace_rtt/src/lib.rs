#![no_std]
use rtt_target::UpChannel;

pub use rtt_target;

static mut RTT_CHANNEL: Option<UpChannel> = None;

pub fn init(channel: UpChannel) {
    unsafe { RTT_CHANNEL = Some(channel) };
}

#[utrace_macros::default_transport]
pub fn write(buf: &[u8]) {
    unsafe {
        if let Some(ref mut channel) = RTT_CHANNEL {
            channel.write(buf);
        }
    }
}
