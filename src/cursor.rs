use std::time::{Duration, Instant};

use crate::mode::Mode;

pub struct Cursor {
    pub x: usize,
    pub y: usize,
    pub visible: bool,
    pub blink_interval: Duration,
    pub last_blink: Instant,
}

impl Cursor {
    pub fn new() -> Self {
        Cursor {
            x: 0,
            y: 0,
            visible: true,
            blink_interval: Duration::from_millis(400),
            last_blink: Instant::now(),
        }
    }

    pub fn blink(&mut self, mode: Mode) {
        let now = Instant::now();

        if now.duration_since(self.last_blink) >= self.blink_interval {
            if mode == Mode::Normal {
                self.visible = true;
            } else {
                self.visible = !self.visible;
                self.last_blink = now;
            }
        }
    }
}
