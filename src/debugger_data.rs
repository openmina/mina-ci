use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct NodeAddress(String);

impl NodeAddress {
    pub fn ip(&self) -> String {
        self.0.split(':').next().unwrap().to_string()
    }

    pub fn port(&self) -> String {
        self.0.split(':').last().unwrap().to_string()
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct DebuggerBlockResponse {
    pub height: usize,
    pub events: Vec<CapturedEvent>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CapturedEvent {
    pub producer_id: String,
    pub hash: String,
    pub block_height: usize,
    pub global_slot: usize,
    pub incoming: bool,
    pub message_kind: String,
    pub message_id: usize, // TODO: doublecheck
    pub time: DebuggerTime,
    pub better_time: DebuggerTime,
    pub latency: Option<DebuggerLatency>,
    pub sender_addr: NodeAddress,
    pub receiver_addr: NodeAddress,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DebuggerTime {
    secs_since_epoch: u64,
    nanos_since_epoch: u64,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct DebuggerLatency {
    secs: u64,
    nanos: u64,
}

impl DebuggerLatency {
    pub fn to_nanoseconds(&self) -> u64 {
        self.nanos + (self.secs * 1_000_000_000)
    }
}

impl DebuggerTime {
    pub fn to_nanoseconds(&self) -> u64 {
        self.nanos_since_epoch + (self.secs_since_epoch * 1_000_000_000)
    }
}

// ---- Cap n Proto stuff ------

pub type DebuggerCpnpResponse = Vec<CpnpCapturedData>;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CpnpCapturedData {
    pub time_microseconds: u64,
    pub real_time_microseconds: u64,
    pub node_address: NodeAddress,
    pub events: Vec<CpnpEvent>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CpnpEvent {
    pub r#type: String,
    pub peer_id: Option<String>,
    pub peer_address: Option<NodeAddress>,
    pub hash: String,
    pub msg: CpnpMessage,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CpnpMessage {
    pub r#type: String,
    pub height: usize,
}
