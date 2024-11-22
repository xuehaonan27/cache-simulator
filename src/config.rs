use serde::{Deserialize, Serialize};

use crate::cache::{ReplacementPolicy, WriteHitPolicy, WriteMissPolicy};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(rename = "CPU_GHZ")]
    pub cpu_ghz: f64,
    #[serde(rename = "using_latency_as_cycles")]
    pub using_latency_as_cycles: bool,
    #[serde(rename = "memory")]
    pub memory: MemoryConfig,
    #[serde(rename = "caches")]
    pub caches: Vec<CacheConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    #[serde(rename = "hit_latency")]
    pub hit_latency: usize,
    #[serde(rename = "bus_latency")]
    pub bus_latency: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    #[serde(rename = "layer")]
    pub layer: usize,
    #[serde(rename = "hit_latency")]
    pub hit_latency: usize,
    #[serde(rename = "bus_latency")]
    pub bus_latency: usize,
    #[serde(rename = "size")]
    pub size: Option<usize>,
    #[serde(rename = "associativity")]
    pub associativity: Option<usize>,
    #[serde(rename = "set_num")]
    pub set_num: Option<usize>,
    #[serde(rename = "block_size")]
    pub block_size: Option<usize>,
    #[serde(rename = "write_hit_policy")]
    pub write_hit_policy: WriteHitPolicy,
    #[serde(rename = "write_miss_policy")]
    pub write_miss_policy: WriteMissPolicy,
    #[serde(rename = "replacement_policy")]
    pub replacement_policy: ReplacementPolicy,
    #[serde(rename = "prefetch")]
    pub prefetch: bool,
    #[serde(rename = "max_streams")]
    pub max_streams: usize,
    #[serde(rename = "stream_buffer_depth")]
    pub stream_buffer_depth: usize,
}
