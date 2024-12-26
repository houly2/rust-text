use std::time::Duration;

use gpui::{ModelContext, Timer};

pub struct BlinkManager {
    blink_interval: Duration,
    enabled: bool,
    paused: bool,
    show: bool,

    epoch: usize,
}

impl BlinkManager {
    pub fn new() -> Self {
        Self {
            blink_interval: Duration::from_millis(500),
            enabled: false,
            paused: false,
            show: false,
            epoch: 0,
        }
    }

    fn blink(&mut self, epoch: usize, cx: &mut ModelContext<Self>) {
        if epoch == self.epoch && self.enabled && !self.paused {
            self.show = !self.show;
            cx.notify();

            let epoch = self.next_epoch();
            let interval = self.blink_interval;
            cx.spawn(|this, mut cx| async move {
                Timer::after(interval).await;
                if let Some(this) = this.upgrade() {
                    this.update(&mut cx, |this, cx| this.blink(epoch, cx)).ok();
                }
            })
            .detach();
        }
    }

    pub fn enable(&mut self, cx: &mut ModelContext<Self>) {
        if self.enabled {
            return;
        }

        self.enabled = true;
        self.show = false;
        self.blink(self.epoch, cx);
    }

    pub fn disable(&mut self) {
        self.show = false;
        self.enabled = false;
    }

    pub fn pause(&mut self, cx: &mut ModelContext<Self>) {
        if !self.show {
            self.show = true;
            cx.notify();
        }

        self.paused = true;

        let epoch = self.next_epoch();
        let interval = self.blink_interval;
        cx.spawn(|this, mut cx| async move {
            Timer::after(interval).await;
            if let Some(this) = this.upgrade() {
                this.update(&mut cx, |this, cx| this.resume(epoch, cx)).ok();
            }
        })
        .detach();
    }

    pub fn resume(&mut self, epoch: usize, cx: &mut ModelContext<Self>) {
        if self.epoch == epoch {
            self.paused = false;
            self.blink(epoch, cx);
        }
    }

    pub fn show(&self) -> bool {
        self.show
    }

    fn next_epoch(&mut self) -> usize {
        self.epoch += 1;
        self.epoch
    }
}
