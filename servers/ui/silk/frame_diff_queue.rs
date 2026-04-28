use alloc::collections::VecDeque;
use crate::ucgm_view::FrameDiff;

pub struct FrameDiffQueue {
    buffer: VecDeque<FrameDiff>,
    capacity: usize,
}

impl FrameDiffQueue {
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: VecDeque::with_capacity(capacity),
            capacity,
        }
    }

    pub fn push(&mut self, diff: FrameDiff) {
        if self.buffer.len() >= self.capacity {
            self.buffer.pop_front();
        }
        self.buffer.push_back(diff);
    }
}
