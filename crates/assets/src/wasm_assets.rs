use anyhow::anyhow;
use gpui::{App, AssetSource, Result, SharedString};
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use wasm_bindgen_futures::spawn_local;

/// Told once for each icon that reaches the cache; [`init`] holds the other end.
static ARRIVALS: Mutex<Option<async_channel::Sender<()>>> = Mutex::new(None);

/// Redraw every window when an on-demand icon reaches the cache.
///
/// `load` answers an error while an icon is in flight, and a frame drawn then
/// drew no icon. GPUI draws only when something asks, so without this the
/// frame that would draw the icon waits for unrelated input. A burst of
/// arrivals is one redraw. A failed fetch asks for nothing, so an icon that
/// never arrives cannot keep the page drawing.
pub fn init(cx: &mut App) {
    let (sender, receiver) = async_channel::unbounded();
    if let Ok(mut arrivals) = ARRIVALS.lock() {
        *arrivals = Some(sender);
    }
    cx.spawn(async move |cx| {
        while receiver.recv().await.is_ok() {
            while receiver.try_recv().is_ok() {}
            cx.refresh();
        }
    })
    .detach();
}

fn arrived() {
    let sender = ARRIVALS.lock().ok().and_then(|arrivals| arrivals.clone());
    if let Some(sender) = sender {
        let _ = sender.try_send(());
    }
}

/// WASM implementation - download assets on-demand
pub struct Assets {
    endpoint: SharedString,
    cache: Arc<RwLock<HashMap<String, Vec<u8>>>>,
    pending: Arc<RwLock<HashMap<String, bool>>>,
}

impl Assets {
    pub fn new(endpoint: impl Into<SharedString>) -> Self {
        Self {
            endpoint: endpoint.into(),
            cache: Arc::new(RwLock::new(HashMap::new())),
            pending: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Default for Assets {
    fn default() -> Self {
        Self::new("")
    }
}

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        if path.is_empty() {
            return Ok(None);
        }

        if path.starts_with("icons/") && path.ends_with(".svg") {
            // Check if already cached
            if let Ok(cache) = self.cache.read() {
                if let Some(data) = cache.get(path) {
                    return Ok(Some(Cow::Owned(data.clone())));
                }
            }

            // Check if download is already pending
            let is_pending = self
                .pending
                .read()
                .map(|p| p.contains_key(path))
                .unwrap_or(false);

            if !is_pending {
                // Mark as pending and start download
                if let Ok(mut pending) = self.pending.write() {
                    pending.insert(path.to_string(), true);
                }

                let url = format!("{}/assets/{}", self.endpoint, path);
                let path_clone = path.to_string();
                let cache = self.cache.clone();
                let pending = self.pending.clone();

                spawn_local(async move {
                    match reqwest::get(&url).await {
                        Ok(response) => {
                            if response.status().is_success() {
                                match response.bytes().await {
                                    Ok(bytes) => {
                                        if let Ok(mut cache) = cache.write() {
                                            cache.insert(path_clone.clone(), bytes.to_vec());
                                        }
                                        arrived();
                                    }
                                    Err(e) => {
                                        log::warn!("Failed to read icon {}: {}", path_clone, e);
                                    }
                                }
                            } else {
                                log::warn!(
                                    "Failed to download icon {}: HTTP {}",
                                    path_clone,
                                    response.status()
                                );
                            }
                        }
                        Err(e) => {
                            log::warn!("Failed to fetch icon {}: {}", path_clone, e);
                        }
                    }

                    // Remove from pending
                    if let Ok(mut pending) = pending.write() {
                        pending.remove(&path_clone);
                    }
                });
            }

            // Return an error; `init` redraws once the icon is cached.
            Err(anyhow!("Wasm assets loading, will be available soon..."))
        } else {
            Ok(None)
        }
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let _ = path;
        Ok(Vec::new())
    }
}
