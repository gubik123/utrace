// Simplistic comma-free self-synchronizing encoder/decoder

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct TracePoint {
    pub delta_t: u32,
    pub id: u8,
}

pub fn encode<W>(tp: TracePoint, writer: W)
where
    W: for<'a> FnOnce(&'a [u8]),
{
    const MAX_TS_SIZE: usize = u32::BITS as usize / 7 + 1;
    let mut outbuf = [0; MAX_TS_SIZE + 1];

    outbuf[0] = tp.id;

    let mut packet_len = 1;
    let mut delta_t = tp.delta_t;

    for i in 0..MAX_TS_SIZE {
        let part = (delta_t & 0x7f) as u8;
        if delta_t == 0 && i > 1 {
            break;
        }
        outbuf[packet_len] = part;
        delta_t >>= 7;
        packet_len += 1;
    }
    outbuf[packet_len - 1] |= 0x80;

    writer(&outbuf[..packet_len]);
}

#[cfg(feature = "std")]
pub struct Decoder {
    queue: std::collections::VecDeque<u8>,
}

#[cfg(feature = "std")]
impl Decoder {
    pub fn new() -> Self {
        Decoder {
            queue: std::collections::VecDeque::new(),
        }
    }

    pub fn push_byte(&mut self, byte: u8) -> Option<TracePoint> {
        let prev_byte = self.queue.back().cloned();
        self.queue.push_back(byte);

        if let Some(prev_byte) = prev_byte {
            if (prev_byte & 0x80 == 0) && (byte & 0x80 != 0) {
                let packet: Vec<_> = self.queue.drain(..).collect();
                let id = packet[0];

                let mut delta_t: u32 = 0;
                for b in packet[1..].iter().rev() {
                    delta_t <<= 7;
                    delta_t |= (b & 0x7f) as u32;
                }

                return Some(TracePoint { id, delta_t });
            }
        }

        None
    }
}

#[cfg(feature = "std")]
impl Default for Decoder {
    fn default() -> Self {
        Decoder::new()
    }
}

#[cfg(test)]
mod test {
    use proptest::prelude::*;
    use std::collections::VecDeque;
    use std::iter::zip;

    use super::*;

    prop_compose! {
        fn arb_tracepoint()(id in any::<u8>(), delta_t in 0u32..(1<<28-1)) -> TracePoint {
            TracePoint {id, delta_t}
        }
    }

    proptest! {
        #[test]
        fn single_enc_dec(pkt in arb_tracepoint()) {

            let mut stream = Vec::new();
            encode(pkt, |b| stream.extend_from_slice(b));

            let mut dec = Decoder::new();

            let mut decoded = None;

            for b in stream {
                decoded = dec.push_byte(b)
            }

            assert_eq!(decoded.unwrap(), pkt);
        }

        #[test]
        fn multi_enc_dec(pkts in prop::collection::vec(arb_tracepoint(), 1..1000)) {

            let mut stream = Vec::new();
            for pkt in pkts.iter() {
                encode(*pkt, |b| stream.extend_from_slice(b));
            }

            let mut dec = Decoder::new();

            let mut decoded = Vec::new();

            for b in stream {
                let d = dec.push_byte(b);
                if let Some(d) = d {
                    decoded.push(d);
                }
            }

            assert_eq!(decoded, pkts);
        }

        #[test]
        fn multi_enc_dec_with_resync(pkts in prop::collection::vec(arb_tracepoint(), 1..1000),
                                    junk in prop::collection::vec(any::<u8>(), 1..1000)) {

            let mut stream = junk;

            encode(TracePoint {id: 0, delta_t: 0}, |b| stream.extend_from_slice(b));
            for pkt in pkts.iter() {
                encode(*pkt, |b| stream.extend_from_slice(b));
            }

            let mut dec = Decoder::new();

            let mut decoded = VecDeque::new();

            for b in stream {
                let d = dec.push_byte(b);
                if let Some(d) = d {
                    decoded.push_back(d);
                }
            }

            while decoded.len() > pkts.len() {
                decoded.pop_front();
            }

            assert_eq!(decoded, pkts);
        }
    }

    #[test]
    fn test_enc_dec() {
        let in_packets = [
            TracePoint {
                id: 10,
                delta_t: 130,
            },
            TracePoint {
                id: 10,
                delta_t: 1030,
            },
            TracePoint {
                id: 10,
                delta_t: 20,
            },
            TracePoint { id: 10, delta_t: 0 },
            TracePoint {
                id: 10,
                delta_t: 130430,
            },
            TracePoint {
                id: 10,
                delta_t: 0x00_20_00_00,
            },
        ];

        let mut serialized: Vec<u8> = Vec::new();

        for p in in_packets {
            encode(p, |b| serialized.extend_from_slice(b));
        }

        let mut deserialized: Vec<TracePoint> = Vec::new();

        let mut dec = Decoder::new();

        for b in serialized {
            let data = dec.push_byte(b);

            if let Some(data) = data {
                deserialized.push(data);
            }
        }

        assert_eq!(deserialized.len(), in_packets.len());

        for (out, inp) in zip(deserialized.iter(), in_packets.iter()) {
            assert_eq!(out, inp);
        }
    }
}

#[cfg(kani)]
mod verification {
    use super::*;

    #[kani::proof]
    #[kani::unwind(6)]
    pub fn check_encoding_length() {
        let mut serialized = Vec::new();
        let tp = TracePoint {
            id: kani::any(),
            delta_t: kani::any(),
        };
        //kani::assume(tp.delta_t < (1 << 14));

        encode(tp, |b| serialized.extend_from_slice(b));

        let mut req_len = 3;
        let out_len = serialized.len();

        kani::cover!(out_len == 3);
        kani::cover!(out_len == 4);
        kani::cover!(out_len == 5);
        kani::cover!(out_len == 6);

        if tp.delta_t >= 1 << 14 {
            req_len += 1;
        }

        if tp.delta_t >= 1 << 21 {
            req_len += 1;
        }

        if tp.delta_t >= 1 << 28 {
            req_len += 1;
        }

        assert_eq!(req_len, out_len);
    }
}
