#[derive(Debug)]
pub struct Bypasser {
    bypass_threshold: usize,
    bypass_patterns: Vec<u64>,
}

impl Bypasser {
    pub fn new(bypass_threshold: usize, bypass_patterns: Vec<u64>) -> Self {
        Bypasser {
            bypass_threshold,
            bypass_patterns,
        }
    }

    pub fn should_bypass(&self, addr: u64, bytes: usize) -> bool {
        if bytes > self.bypass_threshold {
            return true;
        }

        for pattern in &self.bypass_patterns {
            if addr & pattern == *pattern {
                return true;
            }
        }

        false
    }
}
