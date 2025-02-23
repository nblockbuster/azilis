use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

use std::thread::{JoinHandle, spawn};

/// Describes a progress bar.
///
/// This structure is active only when `progress_bar` feature is enabled.
pub struct PBar {
    handle: Option<JoinHandle<()>>,
    cnt: Arc<AtomicU64>,
}

impl PBar {
    pub fn new(max_length: u64, as_bytes: bool) -> Self {
        let cnt = Arc::new(AtomicU64::new(0));

        let cnt2 = cnt.clone();

        Self {
            handle: Some(spawn(move || {
                let mut pbar = pbr::ProgressBar::new(max_length);
                let cnt = cnt2;

                if as_bytes {
                    pbar.set_units(pbr::Units::Bytes);
                }

                let timeout = std::time::Duration::from_millis(30);

                loop {
                    std::thread::sleep(timeout);
                    let loaded = cnt.load(Ordering::Acquire);

                    if loaded == !0 {
                        pbar.finish();
                        break;
                    }

                    pbar.set(loaded);
                }
            })),
            cnt,
        }
    }

    pub fn add(&self, add: u64) {
        self.cnt.fetch_add(add, Ordering::Relaxed);
    }

    pub fn inc(&self) {
        self.add(1);
    }

    pub fn set(&self, value: u64) {
        self.cnt.store(value, Ordering::Relaxed);
    }

    pub fn finish(self) {}
}

impl Drop for PBar {
    fn drop(&mut self) {
        self.cnt.store(!0, Ordering::Release);
        self.handle.take().unwrap().join().unwrap();
    }
}
