use anyhow::Result;
use notify::{Watcher, RecommendedWatcher, RecursiveMode, Event};
use std::path::Path;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver};
use std::time::Duration;

#[allow(dead_code)]
pub struct ImageWatcher {
    watcher: RecommendedWatcher,
    receiver: Receiver<Result<Event, notify::Error>>,
}

#[allow(dead_code)]
impl ImageWatcher {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self> {
        let (tx, rx) = channel();

        let mut watcher = notify::recommended_watcher(move |res| {
            let _ = tx.send(res);
        })?;

        watcher.watch(path.as_ref(), RecursiveMode::Recursive)?;

        Ok(Self { watcher, receiver: rx })
    }

    pub fn wait_for_image(&self, timeout: Duration) -> Option<PathBuf> {
        let start = std::time::Instant::now();
        while start.elapsed() < timeout {
            if let Ok(Ok(event)) = self.receiver.try_recv() {
                if let notify::EventKind::Create(_) = event.kind {
                    for path in event.paths {
                        if self.is_image(&path) {
                            return Some(path);
                        }
                    }
                }
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        None
    }

    fn is_image(&self, path: &Path) -> bool {
        if let Some(ext) = path.extension() {
            let ext = ext.to_str().unwrap_or("").to_lowercase();
            matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp")
        } else {
            false
        }
    }
}
