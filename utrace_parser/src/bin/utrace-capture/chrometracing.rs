use serde::Serialize;
use tokio::{io::AsyncWriteExt, sync::broadcast::Receiver};
use tracing::error;
use utrace_parser::stream_parser::TimestampedTracepoint;

#[derive(Serialize)]
enum EventType {
    #[serde(rename = "B")]
    SpanBegin,
    #[serde(rename = "E")]
    SpanEnd,
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

pub async fn store<'a>(fname: &str, mut chan: Receiver<TimestampedTracepoint<'a>>) {
    if let Ok(mut file) = tokio::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .open(fname)
        .await
    {
        while let Ok(msg) = chan.recv().await {
            if let TimestampedTracepoint::Point {
                timestamp: ts,
                tracepoint: tp,
            } = msg
            {
                let msg_out = Event {
                    name: tp.info.name.to_owned().unwrap(),
                    cat: "Ololo".to_owned(),
                    ty: if tp.info.kind.is_enter() {
                        EventType::SpanBegin
                    } else {
                        EventType::SpanEnd
                    },
                    pid: 1,
                    tid: 1,
                    ts,
                };
                file.write_all(serde_json::to_string(&msg_out).unwrap().as_bytes())
                    .await;
                file.write_all(",\n".as_bytes()).await;
            }
        }
    } else {
        error!("Cannot open file {fname} for writing");
    }
}
