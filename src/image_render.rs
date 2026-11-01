//! Bounded worker for resizing and encoding terminal image protocols.

use std::{
    collections::VecDeque,
    sync::{Arc, Condvar, Mutex},
    thread,
};

use crate::models::DecodedImage;
use ratatui::layout::Size;
use ratatui_image::protocol::Protocol;

mod build;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Halfblocks,
    Iterm2,
}

impl Mode {
    pub fn from_setting(value: &str) -> Self {
        if value.eq_ignore_ascii_case("iterm2") {
            Self::Iterm2
        } else {
            Self::Halfblocks
        }
    }
}

#[derive(Clone)]
pub struct Key {
    pub image: DecodedImage,
    pub size: Size,
    pub zoom: u8,
    pub pan: (u32, u32),
    pub mode: Mode,
}

impl PartialEq for Key {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.image.pixels, &other.image.pixels)
            && self.size == other.size
            && self.zoom == other.zoom
            && self.pan == other.pan
            && self.mode == other.mode
    }
}

struct Queue {
    next: Option<(u64, Key)>,
    generation: u64,
    stopped: bool,
}
type Shared = Arc<(Mutex<Queue>, Condvar)>;
type ResultSlot = Arc<Mutex<Option<(Key, Option<Protocol>)>>>;

/// One running job and one replaceable waiting job; obsolete requests never
/// grow an unbounded queue during rapid navigation or window resizing.
pub struct Worker {
    shared: Shared,
    results: ResultSlot,
    pub requested: Option<Key>,
    pub cache: VecDeque<(Key, Option<Protocol>)>,
}

impl Worker {
    pub fn start() -> std::io::Result<Self> {
        let shared = Arc::new((
            Mutex::new(Queue {
                next: None,
                generation: 0,
                stopped: false,
            }),
            Condvar::new(),
        ));
        let results = Arc::new(Mutex::new(None));
        let worker_results = Arc::clone(&results);
        let worker_shared = Arc::clone(&shared);
        thread::Builder::new()
            .name("ffm-image-render".to_owned())
            .spawn(move || work(&worker_shared, &worker_results))?;
        Ok(Self {
            shared,
            results,
            requested: None,
            cache: VecDeque::new(),
        })
    }

    pub fn request(&mut self, key: Key) {
        if self.requested.as_ref() == Some(&key) {
            return;
        }
        if let Ok(mut queue) = self.shared.0.lock() {
            queue.next = Some((queue.generation, key.clone()));
            self.requested = Some(key);
            self.shared.1.notify_one();
        }
    }

    /// Discard cached and waiting frames and invalidate the running frame.
    /// The worker remains alive, so rapid close/open never spawns extra workers.
    pub fn clear(&mut self) {
        self.requested = None;
        self.cache.clear();
        if let Ok(mut queue) = self.shared.0.lock() {
            queue.generation = queue.generation.wrapping_add(1);
            queue.next = None;
            if let Ok(mut result) = self.results.lock() {
                *result = None;
            }
        }
    }

    pub fn pump(&mut self) -> bool {
        let result = self
            .results
            .lock()
            .ok()
            .and_then(|mut result| result.take());
        let Some((key, protocol)) = result else {
            return false;
        };
        if self.requested.as_ref() == Some(&key) {
            self.requested = None;
        }
        self.cache.push_front((key, protocol));
        self.cache.truncate(2);
        true
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        if let Ok(mut queue) = self.shared.0.lock() {
            queue.stopped = true;
            queue.next = None;
            self.shared.1.notify_one();
        }
    }
}

fn work(shared: &Shared, results: &ResultSlot) {
    while let Some((generation, key)) = next_request(shared) {
        let protocol = build::protocol(&key);
        let Ok(queue) = shared.0.lock() else {
            return;
        };
        if queue.stopped {
            return;
        }
        if queue.generation != generation {
            continue;
        }
        let Ok(mut result) = results.lock() else {
            return;
        };
        *result = Some((key, protocol));
    }
}

fn next_request(shared: &Shared) -> Option<(u64, Key)> {
    let mut queue = shared.0.lock().ok()?;
    while queue.next.is_none() && !queue.stopped {
        queue = shared.1.wait(queue).ok()?;
    }
    if queue.stopped {
        None
    } else {
        queue.next.take()
    }
}

#[cfg(test)]
mod tests;
