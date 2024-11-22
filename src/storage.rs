use std::fmt::Debug;

use crate::{Result, CPU_GHZ, USING_LATENCY_AS_CYCLES};

pub trait Storage: Debug {
    #[allow(unused)]
    fn get_stats(&self) -> StorageStats;
    #[allow(unused)]
    fn get_latency(&self) -> StorageLatency;

    fn get_lower(&self) -> Option<&Box<dyn Storage>>;

    fn calc_print(&self);

    /// - returns: (hit, time)
    fn handle_request(
        &mut self,
        addr: u64,
        bytes: usize,
        read: bool,
        content: &mut [u8],
    ) -> (bool, usize);
}

#[derive(Debug)]
pub struct EmptyStorage {}
impl Storage for EmptyStorage {
    fn get_latency(&self) -> StorageLatency {
        unimplemented!()
    }
    fn get_stats(&self) -> StorageStats {
        unimplemented!()
    }
    fn get_lower(&self) -> Option<&Box<dyn Storage>> {
        None
    }
    fn calc_print(&self) {
        unimplemented!()
    }
    #[allow(unused)]
    fn handle_request(
        &mut self,
        addr: u64,
        bytes: usize,
        read: bool,
        content: &mut [u8],
    ) -> (bool, usize) {
        unimplemented!()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct StorageStats {
    pub access_counter: usize,
    pub miss_num: usize,
    pub access_time: usize,  // In nanoseconds
    pub replace_num: usize,  // Evict old lines
    pub fetch_num: usize,    // Fetch lower layer
    pub prefetch_num: usize, // Prefetch
    pub prefetch_hit: usize, // Prefetch hit
}

impl Default for StorageStats {
    fn default() -> Self {
        Self {
            access_counter: 0,
            miss_num: 0,
            access_time: 0,
            replace_num: 0,
            fetch_num: 0,
            prefetch_num: 0,
            prefetch_hit: 0,
        }
    }
}

impl StorageStats {
    pub fn calc_print(&self) {
        let cpu_hz = unsafe { CPU_GHZ };
        let using_latency_as_cycles = unsafe { USING_LATENCY_AS_CYCLES };
        let miss_rate: f64 = self.miss_num as f64 / self.access_counter as f64;
        let total_time = if using_latency_as_cycles {
            self.access_time as f64 / cpu_hz
        } else {
            self.access_time as f64
        };
        let amat = total_time / self.access_counter as f64;
        let total_cycles = if using_latency_as_cycles {
            self.access_time as f64
        } else {
            cpu_hz * self.access_time as f64
        };
        println!("Access count: {}", self.access_counter);
        println!("Miss count: {}", self.miss_num);
        println!("Miss rate: {}", miss_rate);
        println!("Replace count: {}", self.replace_num);
        println!("Fetch from lower storage count: {}", self.fetch_num);
        println!("Prefetch count: {}", self.prefetch_num);
        println!("Prefetch hit: {}", self.prefetch_hit);
        println!("Total access time: {} ns", total_time);
        println!("Total latency cycles: {}", total_cycles);
        println!("AMAT: {}", amat);
        println!();
    }
}

#[derive(Debug, Clone, Copy)]
pub struct StorageLatency {
    pub hit_latency: usize, // In nanoseconds
    pub bus_latency: usize, // Added to each request
}

impl Default for StorageLatency {
    fn default() -> Self {
        Self {
            hit_latency: 0,
            bus_latency: 0,
        }
    }
}

impl StorageLatency {
    #[allow(unused)]
    pub fn validate(&self) -> Result<()> {
        if self.hit_latency == 0 || self.bus_latency == 0 {
            return Err("Are you expecting an ideal storage? Set the latencies!".into());
        }
        Ok(())
    }
}
