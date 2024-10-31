use std::{fs, path::Path};

pub struct Trace {
    inner: Vec<TraceOp>,
    idx: usize,
}

#[derive(Debug, Clone, Copy)]
pub enum TraceOp {
    Read(u64),
    Write(u64),
}

impl Iterator for Trace {
    type Item = TraceOp;

    fn next(&mut self) -> Option<Self::Item> {
        let idx = self.idx;
        self.idx += 1;
        self.inner.get(idx).cloned()
    }
}

impl Trace {
    pub fn open<S: AsRef<Path>>(path: S) -> Self {
        let path = path.as_ref().as_os_str();
        let content = fs::read_to_string(path).expect("Fail to open the file");

        let inner = content
            .lines()
            .map(|line| {
                let mut tmp = line.split_whitespace();
                (
                    tmp.next().expect("Invalid trace"),
                    tmp.next().expect("Invalid trace"),
                )
            })
            .map(|(rw, addr)| {
                let idx = addr.chars().position(|x| x == 'x');
                let addr = if let Some(idx) = idx {
                    &addr[(idx + 1)..]
                } else {
                    addr
                };
                let addr = u64::from_str_radix(addr, 16).expect("Invalid trace");
                match rw {
                    "r" => TraceOp::Read(addr),
                    "w" => TraceOp::Write(addr),
                    _ => panic!("Invalid trace"),
                }
            })
            .collect();
        Self { inner, idx: 0 }
    }
}
