use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

pub type TracePointId = u8;

/// Kind of enter/exit point pairs
/// - AsyncInstantiation points are emited on enrty and/or exit from instrumentated async fns,
///   hence they cover the lifecycle of async fn from initial call till Future resolution
/// - AsyncPoll point pair covers the execution of respective Future poll function calls
/// - Generic points are emited when instrumentation is inserted by trace_here! macro
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum TracePointPairKind {
    SyncCall,
    AsyncInstantiation,
    AsyncPoll,
    Generic,
}

/// Kind of specific point trace instrumentation point
#[derive(PartialEq, Eq, Debug, Clone, Copy, Hash, Serialize, Deserialize)]
pub enum TracePointKind {
    SyncEnter,
    SyncExit,
    AsyncEnter,
    AsyncExit,
    AsyncPollEnter,
    AsyncPollExit,
    GenericEnter,
    GenericExit,
}

#[derive(PartialEq, Eq, Debug, Clone, Hash, Serialize, Deserialize)]
pub struct TracePointInfo {
    pub kind: TracePointKind,
    pub name: Option<String>,
    pub comment: Option<String>,
    pub skip: Option<u32>,
    pub id: u64,
}

#[derive(Debug, Hash)]
pub struct TracePointDataWithLocation {
    pub info: TracePointInfo,
    pub path: Option<String>,
    pub file_name: Option<String>,
    pub line: Option<u64>,
}

impl TracePointKind {
    pub fn is_enter(&self) -> bool {
        match self {
            TracePointKind::SyncEnter
            | TracePointKind::AsyncEnter
            | TracePointKind::AsyncPollEnter
            | TracePointKind::GenericEnter => true,
            TracePointKind::SyncExit
            | TracePointKind::AsyncExit
            | TracePointKind::AsyncPollExit
            | TracePointKind::GenericExit => false,
        }
    }

    pub fn is_exit(&self) -> bool {
        !self.is_enter()
    }
}

impl From<TracePointKind> for TracePointPairKind {
    fn from(v: TracePointKind) -> TracePointPairKind {
        match v {
            TracePointKind::SyncEnter | TracePointKind::SyncExit => TracePointPairKind::SyncCall,
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
            TracePointPairKind::SyncCall => TracePointKind::SyncEnter,
            TracePointPairKind::AsyncInstantiation => TracePointKind::AsyncEnter,
            TracePointPairKind::AsyncPoll => TracePointKind::AsyncPollEnter,
            TracePointPairKind::Generic => TracePointKind::GenericEnter,
        }
    }

    pub fn exit_point(&self) -> TracePointKind {
        match self {
            TracePointPairKind::SyncCall => TracePointKind::SyncExit,
            TracePointPairKind::AsyncInstantiation => TracePointKind::AsyncExit,
            TracePointPairKind::AsyncPoll => TracePointKind::AsyncPollExit,
            TracePointPairKind::Generic => TracePointKind::GenericExit,
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
