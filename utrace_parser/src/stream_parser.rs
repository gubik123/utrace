use std::collections::HashMap;

use tracing::error;
use utrace::encoding::Decoder;
use utrace_core::trace_point::{TracePointDataWithLocation, TracePointId};

#[derive(Debug, Clone)]
pub enum TimestampedTracepoint<'a> {
    Point {
        timestamp: u64,
        tracepoint: &'a TracePointDataWithLocation,
    },
    Reset,
}

pub struct StreamParser<'a> {
    id_mapping: &'a HashMap<TracePointId, TracePointDataWithLocation>,
    decoder_queue: Decoder,
    timestamp: u64,
}

impl<'a> StreamParser<'a> {
    pub fn new(id_mapping: &'a HashMap<TracePointId, TracePointDataWithLocation>) -> Self {
        StreamParser {
            id_mapping,
            decoder_queue: Decoder::new(),
            timestamp: 0,
        }
    }

    pub fn push_and_parse<'b>(
        &'b mut self,
        data: &'b [u8],
    ) -> impl Iterator<Item = TimestampedTracepoint<'a>> + 'b {
        StreamParserIter {
            inner: self,
            incoming: data,
        }
    }
}

pub struct StreamParserIter<'a, 'b> {
    inner: &'b mut StreamParser<'a>,
    incoming: &'b [u8],
}

impl<'a, 'b> Iterator for StreamParserIter<'a, 'b> {
    type Item = TimestampedTracepoint<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some((&b, rest)) = self.incoming.split_first() {
            self.incoming = rest;
            if let Some(tp) = self.inner.decoder_queue.push_byte(b) {
                println!("{}, {}", tp.id, tp.delta_t);
                if tp.id == 0 && tp.delta_t == 0 {
                    // self.inner.timestamp = 0;
                    return Some(TimestampedTracepoint::Reset);
                }

                self.inner.timestamp += tp.delta_t as u64;
                let data = self.inner.id_mapping.get(&tp.id);
                if let Some(data) = data {
                    return Some(TimestampedTracepoint::Point {
                        timestamp: self.inner.timestamp,
                        tracepoint: data,
                    });
                } else {
                    error!(
                        "Received trace packet with incorrect id={}. Ignoring.",
                        tp.id
                    );
                }
            }
        }

        None
    }
}
