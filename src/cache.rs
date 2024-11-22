use std::{borrow::BorrowMut, time::Instant};

use log::debug;
use serde::{Deserialize, Serialize};

use crate::{
    bypass::Bypasser,
    prefetch::PrefetchStreamManager,
    storage::{Storage, StorageLatency, StorageStats},
    Result,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WriteHitPolicy {
    #[serde(rename = "None")]
    None,
    #[serde(rename = "WriteThrough")]
    WriteThrough,
    #[serde(rename = "WriteBack")]
    WriteBack,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WriteMissPolicy {
    #[serde(rename = "None")]
    None,
    #[serde(rename = "WriteAllocate")]
    WriteAllocate,
    #[serde(rename = "NonWriteAllocate")]
    NonWriteAllocate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReplacementPolicy {
    #[serde(rename = "None")]
    None,
    #[serde(rename = "LRU")]
    LRU,
    #[serde(rename = "PseudoLRU")]
    PseudoLRU,
    #[serde(rename = "Clock")]
    Clock,
}

#[derive(Debug, Clone)]
#[repr(C)]
pub struct CacheLine {
    pub valid: bool,
    pub dirty: bool,
    pub tag: u64,
    pub set: u64,
    pub data: Vec<u8>, // size determined by [`Cache::block_size`]

    pub timestamp: Instant, // for real LRU
    pub used: bool,         // for Clock
}
impl CacheLine {
    pub fn new(block_size: usize) -> Self {
        Self {
            valid: false,
            dirty: false,
            tag: 0,
            set: 0,
            data: vec![0; block_size],
            timestamp: Instant::now(),
            used: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct CacheSet {
    cache_lines: Vec<CacheLine>,
    pseudo_lru_tree: Vec<bool>, // for pseudo LRU algorithm
    clock_pointer: usize,       // for clock algorithm
}
impl CacheSet {
    pub fn new(ways: usize, block_size: usize) -> Self {
        Self {
            cache_lines: vec![CacheLine::new(block_size); ways],
            pseudo_lru_tree: vec![false; ways - 1],
            clock_pointer: 0,
        }
    }
}

#[derive(Debug)]
#[repr(transparent)]
struct CacheInner(Vec<CacheSet>);
impl CacheInner {
    pub fn new(set_num: usize, ways: usize, block_size: usize) -> Self {
        Self(vec![CacheSet::new(ways, block_size); set_num])
    }
}

#[derive(Debug)]
pub struct Cache {
    size: usize,
    associativity: usize,
    set_num: usize,
    block_size: usize,
    write_hit_policy: WriteHitPolicy,
    write_miss_policy: WriteMissPolicy,
    replacement_policy: ReplacementPolicy,
    lower: Box<dyn Storage>,
    latency: StorageLatency,
    stats: StorageStats,
    layer: usize,

    byte_offset_bits: u32,
    byte_offset_mask: u64,
    #[allow(unused)]
    set_index_bits: u32,
    set_index_mask: u64,
    tag_mask: u64,

    inner: CacheInner,

    prefetch_manager: PrefetchStreamManager,
    prefetch: bool,
    bypass: bool,
}

impl Storage for Cache {
    fn get_latency(&self) -> StorageLatency {
        self.latency
    }

    fn get_stats(&self) -> StorageStats {
        self.stats
    }

    fn get_lower(&self) -> Option<&Box<dyn Storage>> {
        Some(&self.lower)
    }

    fn calc_print(&self) {
        println!("---------- L{} Cache ----------", self.layer);
        println!("Size = {}", self.size);
        println!("Associativity = {}", self.associativity);
        println!("Number of set = {}", self.set_num);
        println!("Block size = {}", self.block_size);
        self.stats.calc_print();
    }

    /// - returns: (hit, time)
    fn handle_request(
        &mut self,
        addr: u64,
        bytes: usize,
        read: bool,
        content: &mut [u8],
    ) -> (bool, usize) {
        // debug!("CACHE L{} {addr:#x}", self.layer);
        let mut time = 0;
        self.stats.access_counter += 1;
        // bus latency
        time += self.latency.bus_latency;
        // all access should have this latency
        time += self.latency.hit_latency;

        // calculate
        let byte_offset = addr & self.byte_offset_mask;
        let set_index = addr & self.set_index_mask >> self.byte_offset_bits;
        let tag = addr & self.tag_mask;

        // try to fetch in this cache
        let inner = &mut self.inner.0;
        let cache_set = &mut inner[set_index as usize];
        let cache_line = cache_set
            .cache_lines
            .iter_mut()
            .find(|cache_line| cache_line.valid && cache_line.tag == tag);

        if self.prefetch {
            // if cache miss, then check prefetch streams first
            if cache_line.is_none() {
                if let Some(prefetch_line) = self.prefetch_manager.check_hit(tag, set_index) {
                    // prefetch stream hit
                    // get all the cache lines in this stream into the cache
                    self.stats.prefetch_hit += 1;
                    return (true, time);
                } else {
                    // prefetch stream miss
                    let stride = self.prefetch_manager.record_addr(addr);
                    let depth = self.prefetch_manager.stream_buffer_depth;
                    let stream = self
                        .prefetch_manager
                        .get_stream(stride, self.prefetch_manager.stream_buffer_depth);
                    for i in 0..depth {
                        self.stats.prefetch_num += 1;
                        let prefetch_addr = addr as i64 + (i as i64) * (stride as i64);
                        let prefetch_addr =
                            prefetch_addr as u64 >> self.byte_offset_bits << self.byte_offset_bits;
                        let (hit, pf_time) = self.lower.handle_request(
                            prefetch_addr,
                            self.block_size,
                            true,
                            content,
                        );
                        // pf_time ignored
                        let mut cache_line = CacheLine::new(self.block_size);
                        cache_line.tag = prefetch_addr & self.tag_mask;
                        cache_line.set =
                            prefetch_addr & self.set_index_mask >> self.byte_offset_bits;

                        if stream.buffer.len() >= depth {
                            stream.buffer.pop_front();
                        }
                        stream.buffer.push_back(cache_line);
                    }
                    // fall into below miss logics
                }
            } else {
                // debug!("HIT");
            }
        }

        let res = match (read, cache_line) {
            (true, Some(cache_line)) => {
                // read cache hit
                cache_line.timestamp = Instant::now();
                cache_line.used = true;

                // let copy_range = byte_offset as usize..(byte_offset as usize + bytes);
                // fill the buffer
                // content[copy_range].copy_from_slice(&cache_line.data[0..bytes]);
                (true, time)
            }
            (false, Some(cache_line)) => {
                // write cache hit
                cache_line.timestamp = Instant::now();
                cache_line.used = true;

                match self.write_hit_policy {
                    WriteHitPolicy::None => panic!(),
                    WriteHitPolicy::WriteBack => {
                        // just write here and mark the cache line as dirty
                        cache_line.dirty = true;
                        // let copy_range = byte_offset as usize..(byte_offset as usize + bytes);
                        // cache_line.data[copy_range].copy_from_slice(&content[0..bytes]);
                    }
                    WriteHitPolicy::WriteThrough => {
                        // write here and issue write request to lower
                        cache_line.dirty = true; // useless but set it anyway
                                                 // let copy_range = byte_offset as usize..(byte_offset as usize + bytes);
                                                 // cache_line.data[copy_range].copy_from_slice(&content[0..bytes]);

                        let (wt_hit, wt_time) =
                            self.lower.handle_request(addr, bytes, false, content);
                        time += wt_time;
                    }
                }
                (true, time)
            }
            (true, None) => {
                // read cache miss
                self.stats.miss_num += 1;
                // fetch from lower layer
                self.stats.fetch_num += 1;
                let mut buf = Vec::new();
                buf.resize(self.block_size, 0u8);

                // get an empty cache line for the new block
                let (empty_cache_line, get_time, evicted) = Cache::get_lower_block(
                    cache_set,
                    addr,
                    self.block_size,
                    &mut self.lower,
                    self.replacement_policy,
                    self.write_hit_policy,
                    self.byte_offset_bits,
                    &mut buf,
                );
                time += get_time;
                self.stats.replace_num += if evicted { 1 } else { 0 };

                empty_cache_line.valid = true; // Turn it to valid
                empty_cache_line.dirty = false; // Make it not dirty
                empty_cache_line.tag = tag; // New tag
                empty_cache_line.set = set_index;
                empty_cache_line.timestamp = Instant::now();
                empty_cache_line.used = true;
                // let copy_range = 0..self.block_size;
                // empty_cache_line.data[copy_range.clone()].copy_from_slice(&buf[copy_range]);

                // fill content buffer
                // let copy_range = byte_offset as usize..(byte_offset as usize + bytes);
                // content[0..bytes].copy_from_slice(&empty_cache_line.data[copy_range]);

                (false, time)
            }
            (false, None) => {
                // write cache miss
                self.stats.miss_num += 1;

                match self.write_miss_policy {
                    WriteMissPolicy::None => panic!(),
                    WriteMissPolicy::NonWriteAllocate => {
                        // no write allocation, just issue request to lower storage
                        let (wt_hit, wt_time) =
                            self.lower.handle_request(addr, bytes, false, content);
                        time += wt_time;
                    }
                    WriteMissPolicy::WriteAllocate => {
                        // write allocation
                        // allocate cache line, fetch content from lower storage, make changes and write whole block back
                        // issue request to lower storage to read the content to empty cache line

                        self.stats.fetch_num += 1;
                        let mut buf = Vec::new();
                        buf.resize(self.block_size, 0u8);

                        // Get an empty cache line for the new block
                        let (empty_cache_line, get_time, evicted) = Cache::get_lower_block(
                            cache_set,
                            addr,
                            self.block_size,
                            &mut self.lower,
                            self.replacement_policy,
                            self.write_hit_policy,
                            self.byte_offset_bits,
                            &mut buf,
                        );
                        time += get_time;
                        self.stats.replace_num += if evicted { 1 } else { 0 };

                        empty_cache_line.valid = true;
                        empty_cache_line.dirty = true; // we would make changes ourselves
                        empty_cache_line.tag = tag;
                        empty_cache_line.set = set_index;
                        empty_cache_line.timestamp = Instant::now();
                        empty_cache_line.used = true;
                        // let copy_range = 0..self.block_size;
                        // empty_cache_line.data[copy_range.clone()].copy_from_slice(&buf[copy_range]);

                        // // make changes
                        // let copy_range = byte_offset as usize..(byte_offset as usize + bytes);
                        // empty_cache_line.data[copy_range].copy_from_slice(&content[0..bytes]);
                    }
                }
                (false, time)
            }
        };

        self.stats.access_time += res.1;
        res
    }
}

#[derive(Debug)]
pub struct CacheOptions {
    size: usize,
    associativity: usize,
    set_num: usize,
    block_size: usize,
    write_hit_policy: WriteHitPolicy,
    write_miss_policy: WriteMissPolicy,
    replacement_policy: ReplacementPolicy,
    lower: Option<Box<dyn Storage>>,
    latency: StorageLatency,
    stats: StorageStats,
    layer: usize,
    max_streams: usize,
    stream_buffer_depth: usize,
    prefetch: bool,
    bypass: bool,
}

impl CacheOptions {
    pub fn new() -> Self {
        Self {
            size: 0,
            associativity: 0,
            set_num: 0,
            block_size: 0,
            write_hit_policy: WriteHitPolicy::None,
            write_miss_policy: WriteMissPolicy::None,
            replacement_policy: ReplacementPolicy::None,
            lower: None,
            latency: StorageLatency::default(),
            stats: StorageStats::default(),
            layer: 0,
            max_streams: 8,
            stream_buffer_depth: 4,
            prefetch: false,
            bypass: false,
        }
    }

    pub fn size(mut self, size: usize) -> Self {
        self.size = size;
        self
    }

    pub fn associativity(mut self, asct: usize) -> Self {
        self.associativity = asct;
        self
    }

    pub fn set_num(mut self, set_num: usize) -> Self {
        self.set_num = set_num;
        self
    }

    pub fn block_size(mut self, block_size: usize) -> Self {
        self.block_size = block_size;
        self
    }

    pub fn write_hit_policy(mut self, write_hit_policy: WriteHitPolicy) -> Self {
        self.write_hit_policy = write_hit_policy;
        self
    }

    pub fn write_miss_policy(mut self, write_miss_policy: WriteMissPolicy) -> Self {
        self.write_miss_policy = write_miss_policy;
        self
    }

    pub fn replacement_policy(mut self, replacement_policy: ReplacementPolicy) -> Self {
        self.replacement_policy = replacement_policy;
        self
    }

    pub fn lower(mut self, lower: Box<dyn Storage>) -> Self {
        self.lower = Some(lower);
        self
    }

    pub fn latency(mut self, storage_latency: StorageLatency) -> Self {
        self.latency = storage_latency;
        self
    }

    pub fn stats(mut self, storage_stats: StorageStats) -> Self {
        self.stats = storage_stats;
        self
    }

    pub fn layer(mut self, layer: usize) -> Self {
        self.layer = layer;
        self
    }

    pub fn max_streams(mut self, max_streams: usize) -> Self {
        self.max_streams = max_streams;
        self
    }

    pub fn stream_buffer_depth(mut self, stream_buffer_depth: usize) -> Self {
        self.stream_buffer_depth = stream_buffer_depth;
        self
    }

    pub fn prefetch(mut self, prefetch: bool) -> Self {
        self.prefetch = prefetch;
        self
    }

    pub fn bypass(mut self, bypass: bool) -> Self {
        self.bypass = bypass;
        self
    }

    pub fn build(self) -> Result<Cache> {
        if self.write_miss_policy == WriteMissPolicy::None {
            return Err("Must specify write miss policy".into());
        }
        if self.write_hit_policy == WriteHitPolicy::None {
            return Err("Must specify write hit policy".into());
        }
        if self.replacement_policy == ReplacementPolicy::None {
            return Err("Must specify replacement policy".into());
        }
        if self.lower.is_none() {
            return Err("Must give lower storage".into());
        }
        // self.latency.validate()?;
        let (size, set_num, associativity, block_size) =
            init_cache_config(self.size, self.set_num, self.associativity, self.block_size)?;
        // log2 safe: all values asserted to non-zero and being a power of two

        let byte_offset_bits = block_size.ilog2();
        assert!(2usize.pow(byte_offset_bits) == block_size);
        let byte_offset_mask = (1u64 << byte_offset_bits) - 1;

        let set_index_bits = set_num.ilog2();
        assert!(2usize.pow(set_index_bits) == set_num);
        let set_index_mask = ((1u64 << set_index_bits) - 1) << byte_offset_bits;

        let tag_mask = (!0u64) ^ (byte_offset_mask | set_index_mask);

        /*
        debug!("Cache size = {}", size);
        debug!("Cache number of set = {}", set_num);
        debug!("Cache associativity = {}", associativity);
        debug!("Cache block size = {}", block_size);
        debug!("Cache byte offset bits = {}", byte_offset_bits);
        debug!("Cache byte offset mask = {:#b}", byte_offset_mask);
        debug!("Cache set index bits = {}", set_index_bits);
        debug!("Cache set index mask = {:#b}", set_index_mask);
        debug!("Cache tag mask = {:#b}", tag_mask);
        */

        let inner = CacheInner::new(set_num, associativity, block_size);
        let cache = Cache {
            size,
            associativity,
            set_num,
            block_size,
            write_hit_policy: self.write_hit_policy,
            write_miss_policy: self.write_miss_policy,
            replacement_policy: self.replacement_policy,
            lower: self.lower.unwrap(), // unwrap safe: control flow
            latency: self.latency,
            stats: self.stats,
            layer: self.layer,

            byte_offset_bits,
            byte_offset_mask,
            set_index_bits,
            set_index_mask,
            tag_mask,
            inner,

            prefetch_manager: PrefetchStreamManager::new(
                self.max_streams,
                self.stream_buffer_depth,
            ),
            prefetch: self.prefetch,
            bypass: self.bypass,
        };
        Ok(cache)
    }
}

fn init_cache_config(
    size: usize,
    set_num: usize,
    associativity: usize,
    block_size: usize,
) -> Result<(usize, usize, usize, usize)> {
    let mut zero_num = 0;
    zero_num += if size == 0 { 1 } else { 0 };
    zero_num += if set_num == 0 { 1 } else { 0 };
    zero_num += if associativity == 0 { 1 } else { 0 };
    zero_num += if block_size == 0 { 1 } else { 0 };
    if zero_num > 1 {
        return Err("cache size, set number, asscociativity, block size, three in four must be non-zero value".into());
    }

    macro_rules! checkp2 {
        ($a:expr) => {
            if $a != 0 && !$a.is_power_of_two() {
                return Err(format!(
                    "Invalid cache config {} = {} is not power of two",
                    stringify!($a),
                    $a
                )
                .into());
            }
        };
    }
    checkp2!(size);
    checkp2!(set_num);
    checkp2!(associativity);
    checkp2!(block_size);

    let config = match (size, set_num, associativity, block_size) {
        (0, sn, ascc, bs) => {
            let size = sn * ascc * bs;
            (size, sn, ascc, bs)
        }
        (size, 0, ascc, bs) => {
            let sn = size / (ascc * bs);
            (size, sn, ascc, bs)
        }
        (size, sn, 0, bs) => {
            let ascc = size / (sn * bs);
            (size, sn, ascc, bs)
        }
        (size, sn, ascc, 0) => {
            let bs = size / (sn * ascc);
            (size, sn, ascc, bs)
        }
        (size, sn, ascc, bs) => {
            // check
            if size != sn * ascc * bs {
                return  Err("Inconsistent cache configuration between size, set number, associativity and block size".into());
            }
            (size, sn, ascc, bs)
        }
    };
    Ok(config)
}

impl Cache {
    pub fn get_lower_block<'a>(
        cache_set: &'a mut CacheSet,
        addr: u64,
        block_size: usize,
        lower: &'a mut Box<dyn Storage>,
        replacement_policy: ReplacementPolicy,
        write_hit_policy: WriteHitPolicy,
        byte_offset_bits: u32,
        buf: &mut [u8],
    ) -> (&'a mut CacheLine, usize, bool) {
        let mut time = 0;

        let (rd_hit, rd_time) = lower.handle_request(
            (addr >> byte_offset_bits) << byte_offset_bits,
            block_size,
            true,
            buf,
        );
        time += rd_time;

        let mut evicted = false;

        let found = cache_set
            .cache_lines
            .iter_mut()
            .find(|line| !line.valid)
            .is_some();

        let res: &mut CacheLine = if found {
            let empty_cache_line = cache_set.cache_lines.iter_mut().find(|line| !line.valid);
            empty_cache_line.unwrap()
        } else {
            // evict a cache line
            evicted = true;
            let (empty_cache_line, add_time) = Cache::cache_set_evict_one(
                cache_set,
                lower,
                replacement_policy,
                write_hit_policy,
                byte_offset_bits,
            );
            time += add_time;
            empty_cache_line
        };

        (res, time, evicted)
    }

    fn cache_set_evict_one<'a>(
        cache_set: &'a mut CacheSet,
        lower: &'a mut Box<dyn Storage>,
        replacement_policy: ReplacementPolicy,
        write_hit_policy: WriteHitPolicy,
        byte_offset_bits: u32,
    ) -> (&'a mut CacheLine, usize) {
        let mut time = 0;
        let (the_to_be_evicted_cache_line, idx) = match replacement_policy {
            ReplacementPolicy::None => panic!(),
            ReplacementPolicy::LRU => {
                let mut far_one = 0;
                let mut far_one_time = Instant::now();
                cache_set
                    .cache_lines
                    .iter()
                    .enumerate()
                    .for_each(|(idx, line)| {
                        if line.timestamp < far_one_time {
                            far_one = idx;
                            far_one_time = line.timestamp;
                        }
                    });
                let the_to_be_evicted_cache_line = &mut cache_set.cache_lines[far_one];

                (the_to_be_evicted_cache_line, far_one)
            }
            ReplacementPolicy::PseudoLRU => {
                let mut node_index = 0;
                let num_lines = cache_set.cache_lines.len();
                let num_nodes = num_lines - 1;

                // Traverse the tree to find the leaf to evict
                while node_index < num_nodes {
                    let left_child = 2 * node_index + 1;
                    let right_child = 2 * node_index + 2;

                    if cache_set.pseudo_lru_tree[node_index] {
                        node_index = left_child;
                    } else {
                        node_index = right_child;
                    }
                }

                // Convert the leaf node index to the cache line index
                let line_index = node_index - num_nodes;
                let the_to_be_evicted_cache_line = &mut cache_set.cache_lines[line_index];

                // Update the tree to reflect the eviction
                let mut update_index = node_index;
                while update_index > 0 {
                    let parent_index = (update_index - 1) / 2;
                    cache_set.pseudo_lru_tree[parent_index] = update_index % 2 == 0;
                    update_index = parent_index;
                }

                (the_to_be_evicted_cache_line, line_index)
            }
            ReplacementPolicy::Clock => {
                let mut idx = cache_set.clock_pointer;
                while cache_set.cache_lines[idx].used {
                    cache_set.cache_lines[idx].used = false;
                    idx = (idx + 1) % cache_set.cache_lines.len();
                }
                cache_set.clock_pointer = (idx + 1) % cache_set.cache_lines.len();
                let the_to_be_evicted_cache_line = &mut cache_set.cache_lines[idx];

                (the_to_be_evicted_cache_line, idx)
            }
        };

        // if write back and this cache line is dirty
        if the_to_be_evicted_cache_line.dirty && write_hit_policy == WriteHitPolicy::WriteBack {
            // we should write back
            // calculate write back address
            let addr = the_to_be_evicted_cache_line.tag | ((idx as u64) << byte_offset_bits);
            // the content should be data in this cache line
            let content = &mut the_to_be_evicted_cache_line.data;
            let (wb_hit, wb_time) =
                lower.handle_request(addr, 2usize.pow(byte_offset_bits), false, content);
            time += wb_time;
        }
        // already written back
        (the_to_be_evicted_cache_line, time)
    }
}
