use serde::Serialize;
use std::collections::HashMap;
use std::time::SystemTime;
use std::time::UNIX_EPOCH;
use tokio::{io::AsyncWriteExt, sync::broadcast::Receiver};
use tracing::error;
use utrace_parser::stream_parser::TimestampedTracepoint;

#[derive(Serialize, PartialEq)]
enum EventType {
    #[serde(rename = "B")]
    SpanBegin,
    #[serde(rename = "E")]
    SpanEnd,
    #[serde(rename = "i")]
    Instant,
}

enum DrawingTypes {
    Span,
    Instant,
}

#[derive(Serialize, PartialEq)]
enum ArrowType {
    #[serde(rename = "s")]
    ArrowStart,
    #[serde(rename = "t")]
    ArrowStep,
}

#[derive(Serialize)]
struct Event {
    name: String,
    cat: String,
    #[serde(rename = "ph")]
    ty: EventType,
    pid: u32,
    tid: u32,
    ts: u64,
}

#[derive(Serialize)]
struct ArrowEvent {
    name: String,
    cat: String,
    #[serde(rename = "ph")]
    ty: ArrowType,
    pid: u32,
    tid: u32,
    ts: u64,
    id: u32,
    bp: String,
}

struct TraceEntry {
    last_timestamp: u64,
    unique_id: u32,
}

pub struct Store {
    hm: HashMap<u64, DrawingTypes>,
}

impl Store {
    pub fn new(tp_map: &HashMap<u8, utrace_core::trace_point::TracePointDataWithLocation>) -> Self {
        let mut hm = HashMap::new();

        for tp in tp_map.values() {
            let hash_id = tp.info.id;

            match tp.info.kind {
                utrace_core::trace_point::TracePointKind::AsyncEnter => (),
                utrace_core::trace_point::TracePointKind::AsyncExit => (),
                _ => {
                    hm.entry(hash_id)
                        .and_modify(|w| *w = DrawingTypes::Span)
                        .or_insert(DrawingTypes::Instant);
                }
            }
        }

        Store { hm }
    }

    pub async fn store<'a>(&self, fname: &str, mut chan: Receiver<TimestampedTracepoint<'a>>) {
        let mut events: HashMap<String, TraceEntry> = HashMap::new();
        let mut unique_id_counter: u32 = 0;

        'reset_loop: loop {
            let start = SystemTime::now();
            let since_the_epoch = start
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards");
            let current_time = since_the_epoch.as_secs();
            let fname_ts = format!("{}_{}.json", fname.to_owned(), current_time);

            if let Ok(mut file) = tokio::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .open(fname_ts)
                .await
            {
                let _ = file.write_all(b"[ \n").await;
                while let Ok(msg) = chan.recv().await {
                    match msg {
                        TimestampedTracepoint::Point {
                            timestamp: ts,
                            tracepoint: tp,
                        } => {
                            let mut arrow: Option<ArrowEvent> = None;
                            let mut arrow_type = ArrowType::ArrowStep;
                            let name = tp.info.name.to_owned().unwrap();

                            let event_type: EventType = if tp.info.kind.is_enter() {
                                let existing_event: &mut TraceEntry;

                                // If new task execution, but prev was not dropped
                                if events.contains_key(&name)
                                    && tp.info.kind
                                        != utrace_core::trace_point::TracePointKind::AsyncPollEnter
                                {
                                    events.remove(&name);
                                }

                                if events.contains_key(&name) {
                                    existing_event = events.get_mut(&name).unwrap();

                                    // If the event already exists, update the timestamp
                                    existing_event.last_timestamp = ts;
                                } else {
                                    // New event, insert into the HashMap
                                    events.insert(
                                        name.clone(),
                                        TraceEntry {
                                            last_timestamp: ts,
                                            unique_id: unique_id_counter,
                                        },
                                    );

                                    existing_event = events.get_mut(&name).unwrap();
                                    arrow_type = ArrowType::ArrowStart;

                                    unique_id_counter += 1;
                                }

                                let ret_event_type: EventType;

                                match tp.info.kind {
                                    utrace_core::trace_point::TracePointKind::AsyncEnter => {
                                        ret_event_type = EventType::Instant;
                                    }
                                    _ => match self.hm.get(&tp.info.id) {
                                        Some(DrawingTypes::Span) => {
                                            ret_event_type = EventType::SpanBegin
                                        }
                                        _ => ret_event_type = EventType::Instant,
                                    },
                                }

                                if ret_event_type == EventType::Instant
                                    || arrow_type == ArrowType::ArrowStart
                                {
                                    arrow = Some(ArrowEvent {
                                        name: name.clone(),
                                        cat: name.clone(),
                                        ty: arrow_type,
                                        pid: 1,
                                        tid: 1,
                                        ts,
                                        id: existing_event.unique_id,
                                        bp: "e".to_owned(),
                                    });
                                }

                                ret_event_type
                            } else {
                                let end_id = if events.contains_key(&name) {
                                    events.get_mut(&name).unwrap().last_timestamp = ts;
                                    events.get_mut(&name).unwrap().unique_id
                                } else {
                                    unique_id_counter + 1
                                };

                                arrow = Some(ArrowEvent {
                                    name: name.clone(),
                                    cat: name.clone(),
                                    ty: ArrowType::ArrowStep,
                                    pid: 1,
                                    tid: 1,
                                    ts,
                                    id: end_id,
                                    bp: "e".to_owned(),
                                });

                                match tp.info.kind {
                                    utrace_core::trace_point::TracePointKind::AsyncExit => {
                                        events.remove(&name);
                                        EventType::Instant
                                    }
                                    _ => match self.hm.get(&tp.info.id) {
                                        Some(DrawingTypes::Span) => EventType::SpanEnd,
                                        _ => EventType::Instant,
                                    },
                                }
                            };

                            let msg_out = Event {
                                name,
                                cat: tp.info.kind.to_string(),
                                ty: event_type,
                                pid: 1,
                                tid: 1,
                                ts,
                            };
                            let _ = file
                                .write_all(serde_json::to_string(&msg_out).unwrap().as_bytes())
                                .await;
                            let _ = file.write_all(",\n".as_bytes()).await;

                            if let Some(arrow_event) = arrow {
                                let _ = file
                                    .write_all(
                                        serde_json::to_string(&arrow_event).unwrap().as_bytes(),
                                    )
                                    .await;
                                let _ = file.write_all(",\n".as_bytes()).await;
                            }
                        }
                        TimestampedTracepoint::Reset => {
                            let _ = file.write_all(b"]").await;
                            continue 'reset_loop;
                        }
                    }
                }
                // Properly close the JSON array
            } else {
                error!("Cannot open file {fname} for writing");
            }
        }
    }
}
