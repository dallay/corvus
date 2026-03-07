use anyhow::{Context, Result};
use notify::{Config, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub struct PromptHotReload {
    prompt_path: PathBuf,
    current_prompt: Arc<Mutex<String>>,
    _watcher: RecommendedWatcher,
}

impl PromptHotReload {
    pub fn new(prompt_path: &Path) -> Result<Self> {
        let prompt_path = prompt_path.to_path_buf();
        let current_prompt = Arc::new(Mutex::new(
            std::fs::read_to_string(&prompt_path).unwrap_or_default(),
        ));
        let prompt_copy = Arc::clone(&current_prompt);
        let watched_path = prompt_path.clone();

        let mut watcher = RecommendedWatcher::new(
            move |result: notify::Result<notify::Event>| {
                if let Ok(event) = result {
                    if matches!(event.kind, EventKind::Modify(_) | EventKind::Create(_)) {
                        let relevant = event.paths.iter().any(|path| {
                            path == &watched_path
                                || path.file_name().and_then(|name| name.to_str())
                                    == watched_path.file_name().and_then(|name| name.to_str())
                        });
                        if relevant {
                            if let Ok(contents) = std::fs::read_to_string(&watched_path) {
                                if let Ok(mut guard) = prompt_copy.lock() {
                                    *guard = contents;
                                }
                            }
                        }
                    }
                }
            },
            Config::default(),
        )
        .context("failed to initialize prompt watcher")?;

        let parent = prompt_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        watcher
            .watch(&parent, RecursiveMode::NonRecursive)
            .with_context(|| format!("failed to watch prompt dir: {}", parent.display()))?;

        Ok(Self {
            prompt_path,
            current_prompt,
            _watcher: watcher,
        })
    }

    pub fn latest_prompt(&self) -> String {
        self.current_prompt
            .lock()
            .map(|value| value.clone())
            .unwrap_or_default()
    }

    pub fn wait_for_prompt(&self, expected: &str, timeout: Duration) -> Result<String> {
        let started = Instant::now();
        loop {
            let current = self.latest_prompt();
            if current.contains(expected) {
                return Ok(current);
            }
            if started.elapsed() > timeout {
                anyhow::bail!(
                    "timed out waiting for prompt update at {}",
                    self.prompt_path.display()
                );
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }
}
