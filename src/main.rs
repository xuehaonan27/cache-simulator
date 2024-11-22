use std::fs;

use cache::CacheOptions;
use clap::Parser;
use config::Config;
use log::error;
use memory::MemoryOptions;
use storage::{EmptyStorage, Storage, StorageLatency, StorageStats};
use trace::{Trace, TraceOp};

mod cache;
mod config;
mod logger;
mod memory;
mod prefetch;
mod storage;
mod trace;

pub type BoxDynError = Box<dyn std::error::Error>;
pub type Result<T> = std::result::Result<T, BoxDynError>;

static mut CPU_GHZ: f64 = 0.0;
static mut USING_LATENCY_AS_CYCLES: bool = false;

#[derive(clap::Parser, Debug)]
struct Args {
    /// Whether should print debug information
    #[arg(long)]
    debug: bool,

    /// Path to the trace to be loaded
    #[arg(short, long)]
    trace: String,

    /// Path to the configuration of memory hierarchy
    #[arg(short, long)]
    config: String,
}

fn main() -> Result<()> {
    logger::init();
    let args = Args::parse();
    let trace_path = args.trace;
    let config_path = args.config;

    let tracer = Trace::open(trace_path);
    let config = fs::read_to_string(config_path)?;
    let mut config: Config = serde_json::from_str(&config)?;

    unsafe { CPU_GHZ = config.cpu_ghz };
    unsafe { USING_LATENCY_AS_CYCLES = config.using_latency_as_cycles };

    let memory = MemoryOptions::new()
        .latency(StorageLatency {
            hit_latency: config.memory.hit_latency,
            bus_latency: config.memory.bus_latency,
        })
        .stats(StorageStats::default())
        .build()?;

    config.caches.sort_by_key(|x| x.layer);
    let mut caches = Vec::new();
    for (idx, cache_config) in config.caches.iter().enumerate() {
        if idx + 1 != cache_config.layer {
            let msg = "Caches should be configured with layer number 1, 2, 3... which should start from 0 and not skipped";
            error!("{msg}");
            return Err(msg.into());
        }
        let mut cache = CacheOptions::new()
            .write_hit_policy(cache_config.write_hit_policy)
            .write_miss_policy(cache_config.write_miss_policy)
            .replacement_policy(cache_config.replacement_policy)
            .latency(StorageLatency {
                hit_latency: cache_config.hit_latency,
                bus_latency: cache_config.bus_latency,
            })
            .stats(StorageStats::default())
            .layer(cache_config.layer);

        if let Some(size) = cache_config.size {
            cache = cache.size(size);
        }
        if let Some(associativity) = cache_config.associativity {
            cache = cache.associativity(associativity);
        }
        if let Some(set_num) = cache_config.set_num {
            cache = cache.set_num(set_num);
        }
        if let Some(block_size) = cache_config.block_size {
            cache = cache.block_size(block_size);
        }
        caches.push(cache);
    }

    let mut l1_cache: Box<dyn Storage> = Box::new(memory);
    for cache_options in caches.into_iter().rev() {
        let lower = std::mem::replace(&mut l1_cache, Box::new(EmptyStorage {}));
        let cache = cache_options.lower(lower).build()?;
        l1_cache = Box::new(cache);
    }

    let mut content = [0u8; 4096];
    tracer.for_each(|trace_op| match trace_op {
        TraceOp::Read(addr) => {
            l1_cache.handle_request(addr, 1, true, &mut content);
        }
        TraceOp::Write(addr) => {
            l1_cache.handle_request(addr, 1, false, &mut content);
        }
    });

    let mut storage_ref = &l1_cache;
    storage_ref.calc_print();
    while let Some(storage) = storage_ref.get_lower() {
        storage.calc_print();
        storage_ref = &storage;
    }

    Ok(())
}
