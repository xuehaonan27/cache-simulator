use std::collections::HashMap;

use log::debug;

use crate::{
    storage::{Storage, StorageLatency, StorageStats},
    Result,
};

const PAGE_NUM_MASK: u64 = !0xFFF;

#[derive(Debug)]
#[repr(transparent)]
struct Page([u8; 4096]);
impl Page {
    pub fn empty() -> Self {
        Self([0; 4096])
    }
}

#[derive(Debug)]
#[repr(transparent)]
struct MemoryInner(HashMap<u64, Page>);

#[derive(Debug)]
pub struct Memory {
    latency: StorageLatency,
    stats: StorageStats,

    inner: MemoryInner,
}

impl Storage for Memory {
    fn get_stats(&self) -> StorageStats {
        self.stats
    }

    fn get_latency(&self) -> StorageLatency {
        self.latency
    }

    fn get_lower(&self) -> Option<&Box<dyn Storage>> {
        None
    }

    fn calc_print(&self) {
        println!("---------- Main Memory ----------");
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
        // debug!("MEMORY {addr:#x}");
        self.stats.access_counter += 1;
        let time = self.latency.hit_latency + self.latency.bus_latency;

        let start_page_num = addr & PAGE_NUM_MASK;
        let end_addr = addr + bytes as u64;

        let mut idx = 0;

        for page_num in (start_page_num..end_addr).step_by(0x1000) {
            if !self.inner.0.contains_key(&page_num) {
                self.inner.0.insert(page_num, Page::empty());
            }
            let page = self.inner.0.get_mut(&page_num).unwrap(); // unwrap safe
            let page_lower_bound = page_num;
            let page_upper_bound = page_num + 0x1000;
            let access_lower_bound = (page_lower_bound.max(addr) - page_lower_bound) as usize;
            let access_upper_bound = (page_upper_bound.min(end_addr) - page_lower_bound) as usize;

            let slice_bytes = access_upper_bound - access_lower_bound;
            // if read {
            //     content[idx..idx + slice_bytes]
            //         .copy_from_slice(&page.0[access_lower_bound..access_upper_bound]);
            // } else {
            //     page.0[access_lower_bound..access_upper_bound]
            //         .copy_from_slice(&content[idx..idx + slice_bytes]);
            // }
            idx += slice_bytes;
        }

        self.stats.access_time += time;

        (true, time)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MemoryOptions {
    latency: StorageLatency,
    stats: StorageStats,
}

impl MemoryOptions {
    pub fn new() -> Self {
        Self {
            latency: StorageLatency::default(),
            stats: StorageStats::default(),
        }
    }

    pub fn latency(mut self, storage_latency: StorageLatency) -> Self {
        self.latency = storage_latency;
        self
    }

    pub fn stats(mut self, storage_stats: StorageStats) -> Self {
        self.stats = storage_stats;
        self
    }

    pub fn build(self) -> Result<Memory> {
        let inner = MemoryInner(HashMap::new());
        let memory = Memory {
            latency: self.latency,
            stats: self.stats,
            inner,
        };
        Ok(memory)
    }
}
