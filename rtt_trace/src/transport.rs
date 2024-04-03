pub trait Transport {
    fn write(&mut self, buf: &[u8]);
}

pub(crate) struct GlobalTransport;

impl Transport for GlobalTransport {
    fn write(&mut self, buf: &[u8]) {}
}
