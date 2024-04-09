use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Kind of enter/exit point pairs
/// - AsyncInstantiation points are emited on enrty and/or exit from instrumentated async fns,
///   hence they cover the lifecycle of async fn from initial call till Future resolution
/// - AsyncPoll point pair covers the execution of respective Future poll function calls
/// - Generic points are emited when instrumentation is inserted by trace_here! macro
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum TracePointPairKind {
    AsyncInstantiation,
    AsyncPoll,
    Generic,
}

/// Kind of specific point trace instrumentation point
#[derive(PartialEq, Eq, Debug, Clone, Copy, Serialize, Deserialize)]
pub enum TracePointKind {
    AsyncEnter,
    AsyncExit,
    AsyncPollEnter,
    AsyncPollExit,
    GenericEnter,
    GenericExit,
}

#[derive(PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub struct TracePointInfo {
    pub kind: TracePointKind,
    pub name: Option<String>,
    pub comment: Option<String>,
    pub skip: Option<u32>,
    pub id: u64,
}

impl Into<TracePointPairKind> for TracePointKind {
    fn into(self) -> TracePointPairKind {
        match self {
            TracePointKind::AsyncEnter | TracePointKind::AsyncExit => {
                TracePointPairKind::AsyncInstantiation
            }
            TracePointKind::AsyncPollEnter | TracePointKind::AsyncPollExit => {
                TracePointPairKind::AsyncPoll
            }
            TracePointKind::GenericEnter | TracePointKind::GenericExit => {
                TracePointPairKind::Generic
            }
        }
    }
}

impl TracePointPairKind {
    pub fn enter_point(&self) -> TracePointKind {
        match self {
            TracePointPairKind::AsyncInstantiation => TracePointKind::AsyncEnter,
            TracePointPairKind::AsyncPoll => TracePointKind::AsyncPollEnter,
            TracePointPairKind::Generic => TracePointKind::GenericEnter,
        }
    }

    pub fn exit_point(&self) -> TracePointKind {
        match self {
            TracePointPairKind::AsyncInstantiation => TracePointKind::AsyncExit,
            TracePointPairKind::AsyncPoll => TracePointKind::AsyncPollExit,
            TracePointPairKind::Generic => TracePointKind::GenericEnter,
        }
    }
}

impl TracePointInfo {
    pub fn to_escaped_string(&self) -> String {
        let serialized =
            serde_json::to_string(self).expect("Internal error during TracePoint serialization");
        escape(&serialized)
    }

    pub fn from_mangled_string<T>(s: T) -> Result<Self>
    where
        T: AsRef<str>,
    {
        let serialized = unescape(s.as_ref());
        serde_json::from_str::<Self>(&serialized)
            .context("Malformed JSON deserialization attempt for TracePointInfo")
    }
}

fn escape(inp: &str) -> String {
    inp.to_owned()
}

fn unescape(inp: &str) -> String {
    inp.to_owned()
}
