use std::{
    collections::{HashMap, VecDeque},
    time::Instant,
};

use log::debug;

use crate::cache::CacheLine;

#[derive(Debug)]
pub struct PrefetchStream {
    pub buffer: VecDeque<CacheLine>,
    timestamp: Instant,
    stride: i64,
    depth: usize,
}

impl PrefetchStream {
    pub fn new(stride: i64, depth: usize) -> Self {
        PrefetchStream {
            buffer: VecDeque::with_capacity(depth),
            timestamp: Instant::now(),
            stride,
            depth,
        }
    }

    pub fn check_hit(&self, tag: u64, set: u64) -> Option<&CacheLine> {
        self.buffer
            .iter()
            .find(|&cache_line| cache_line.tag == tag && cache_line.set == set)
    }
}

#[derive(Debug)]
pub struct PrefetchStreamManager {
    access_history: Vec<(u64, i64)>, // (address, stride)
    streams: HashMap<i64, PrefetchStream>,
    max_streams: usize,
    pub stream_buffer_depth: usize,
}

impl PrefetchStreamManager {
    pub fn new(max_streams: usize, stream_buffer_depth: usize) -> Self {
        PrefetchStreamManager {
            access_history: Vec::new(),
            streams: HashMap::new(),
            max_streams,
            stream_buffer_depth,
        }
    }

    pub fn get_stream(&mut self, stride: i64, depth: usize) -> &mut PrefetchStream {
        if !self.streams.contains_key(&stride) {
            if self.streams.len() >= self.max_streams {
                // Evict the least recently used stream
                let mut min_timestamp = Instant::now();
                let mut min_stride = 0;
                for (stride, stream) in self.streams.iter() {
                    if stream.buffer.back().unwrap().timestamp < min_timestamp {
                        min_timestamp = stream.buffer.back().unwrap().timestamp;
                        min_stride = *stride;
                    }
                }
                self.streams.remove(&min_stride);
            }
            self.streams
                .insert(stride, PrefetchStream::new(stride, depth));
        }
        let res = self.streams.get_mut(&stride).unwrap();
        res.timestamp = Instant::now(); // update timestamp
        res
    }

    pub fn check_hit(&self, tag: u64, set: u64) -> Option<&CacheLine> {
        for stream in self.streams.values() {
            if let Some(cache_line) = stream.check_hit(tag, set) {
                return Some(cache_line);
            }
        }
        None
    }

    pub fn record_addr(&mut self, addr: u64) -> i64 {
        let mut stride = -1;
        for (prev_addr, prev_stride) in self.access_history.iter().rev() {
            let cur_stride = addr as i64 - *prev_addr as i64;
            if (*prev_stride == -1 || cur_stride == *prev_stride) && cur_stride.abs() <= 64 {
                stride = cur_stride;
                break;
            }
        }

        self.access_history.push((addr, stride));
        stride
    }
}
