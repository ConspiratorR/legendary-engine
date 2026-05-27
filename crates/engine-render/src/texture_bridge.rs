use crate::texture_store::TextureStore;
use engine_asset::asset::{Handle, HandleId};
use engine_asset::types::Texture;
use std::collections::HashMap;
use std::sync::Arc;

/// Unique identifier for a registered event listener.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ListenerId(usize);

type Listener<T> = (ListenerId, Arc<dyn Fn(&T) + Send + Sync>);

/// Synchronous publish/subscribe event channel.
/// Duplicates engine_core::event::EventChannel to avoid circular dependency.
pub struct EventChannel<T: Send + 'static> {
    listeners: Vec<Listener<T>>,
    next_id: usize,
}

impl<T: Send + 'static> EventChannel<T> {
    pub fn new() -> Self {
        Self {
            listeners: Vec::new(),
            next_id: 0,
        }
    }

    pub fn subscribe(&mut self, handler: impl Fn(&T) + Send + Sync + 'static) -> ListenerId {
        let id = ListenerId(self.next_id);
        self.next_id += 1;
        self.listeners.push((id, Arc::new(handler)));
        id
    }

    pub fn emit(&self, event: &T) {
        for (_, listener) in &self.listeners {
            listener(event);
        }
    }
}

impl<T: Send + 'static> Default for EventChannel<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// State of a texture load request.
#[derive(Clone, Debug)]
pub enum LoadState {
    Pending,
    Ready(u64),
    Failed(String),
}

/// Fired when a texture finishes loading.
#[derive(Clone, Debug)]
pub struct TextureLoaded {
    pub handle_id: HandleId,
    pub result: Result<u64, String>,
}

struct LoadRequest {
    handle_id: HandleId,
    path: String,
}

enum LoadResult {
    Success {
        handle_id: HandleId,
        pixels: Vec<u8>,
        width: u32,
        height: u32,
    },
    Failure {
        handle_id: HandleId,
        error: String,
    },
}

/// Bridge between asset system Handle<Texture> and render system TextureStore.
/// Loads textures asynchronously and uploads to GPU on flush().
pub struct TextureBridge {
    handle_to_id: HashMap<HandleId, u64>,
    states: HashMap<HandleId, LoadState>,
    completed_queue: crossbeam_channel::Receiver<LoadResult>,
    load_sender: crossbeam_channel::Sender<LoadRequest>,
    texture_store: TextureStore,
    pub on_loaded: EventChannel<TextureLoaded>,
}

impl TextureBridge {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("sprite_texture_bind_group_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let (load_tx, load_rx) = crossbeam_channel::unbounded::<LoadRequest>();
        let (done_tx, done_rx) = crossbeam_channel::unbounded::<LoadResult>();

        std::thread::spawn(move || {
            for req in load_rx {
                let result = std::fs::read(&req.path)
                    .map_err(|e| e.to_string())
                    .and_then(|bytes| image::load_from_memory(&bytes).map_err(|e| e.to_string()));

                let load_result = match result {
                    Ok(img) => {
                        let rgba = img.to_rgba8();
                        let (w, h) = rgba.dimensions();
                        LoadResult::Success {
                            handle_id: req.handle_id,
                            pixels: rgba.into_raw(),
                            width: w,
                            height: h,
                        }
                    }
                    Err(e) => LoadResult::Failure {
                        handle_id: req.handle_id,
                        error: e,
                    },
                };
                if done_tx.send(load_result).is_err() {
                    break;
                }
            }
        });

        Self {
            handle_to_id: HashMap::new(),
            states: HashMap::new(),
            completed_queue: done_rx,
            load_sender: load_tx,
            texture_store: TextureStore::new(device, queue, texture_layout),
            on_loaded: EventChannel::new(),
        }
    }

    pub fn request(&mut self, handle: &Handle<Texture>, path: &str) {
        let handle_id = HandleId::from_handle(handle);
        if self.states.contains_key(&handle_id) {
            return;
        }
        self.states.insert(handle_id, LoadState::Pending);
        let _ = self.load_sender.send(LoadRequest {
            handle_id,
            path: path.to_string(),
        });
    }

    pub fn flush(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        while let Ok(result) = self.completed_queue.try_recv() {
            match result {
                LoadResult::Success {
                    handle_id,
                    pixels,
                    width,
                    height,
                } => match self
                    .texture_store
                    .load_from_bytes(device, queue, &pixels, width, height)
                {
                    Ok(texture_id) => {
                        self.handle_to_id.insert(handle_id, texture_id);
                        self.states.insert(handle_id, LoadState::Ready(texture_id));
                        self.on_loaded.emit(&TextureLoaded {
                            handle_id,
                            result: Ok(texture_id),
                        });
                    }
                    Err(e) => {
                        let msg = e.to_string();
                        self.states
                            .insert(handle_id, LoadState::Failed(msg.clone()));
                        self.on_loaded.emit(&TextureLoaded {
                            handle_id,
                            result: Err(msg),
                        });
                    }
                },
                LoadResult::Failure { handle_id, error } => {
                    self.states
                        .insert(handle_id, LoadState::Failed(error.clone()));
                    self.on_loaded.emit(&TextureLoaded {
                        handle_id,
                        result: Err(error),
                    });
                }
            }
        }
    }

    pub fn resolve(&self, handle: &Handle<Texture>) -> u64 {
        let handle_id = HandleId::from_handle(handle);
        self.handle_to_id.get(&handle_id).copied().unwrap_or(0)
    }

    pub fn state(&self, handle: &Handle<Texture>) -> Option<&LoadState> {
        let handle_id = HandleId::from_handle(handle);
        self.states.get(&handle_id)
    }

    pub fn texture_store(&self) -> &TextureStore {
        &self.texture_store
    }

    pub fn texture_store_mut(&mut self) -> &mut TextureStore {
        &mut self.texture_store
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_device() -> (wgpu::Device, wgpu::Queue) {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: None,
            force_fallback_adapter: false,
        }))
        .unwrap();
        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                label: None,
                memory_hints: wgpu::MemoryHints::Performance,
            },
            None,
        ))
        .unwrap();
        (device, queue)
    }

    #[test]
    fn test_resolve_unknown_returns_fallback() {
        let (device, queue) = test_device();
        let bridge = TextureBridge::new(&device, &queue);
        let tex = Texture {
            id: "test".into(),
            width: 1,
            height: 1,
            data: vec![255, 0, 0, 255],
            channels: 4,
        };
        let handle = Handle::new(tex);
        assert_eq!(bridge.resolve(&handle), 0);
    }

    #[test]
    fn test_request_sets_pending_state() {
        let (device, queue) = test_device();
        let mut bridge = TextureBridge::new(&device, &queue);
        let tex = Texture {
            id: "test".into(),
            width: 1,
            height: 1,
            data: vec![255, 0, 0, 255],
            channels: 4,
        };
        let handle = Handle::new(tex);
        bridge.request(&handle, "nonexistent_path.png");
        match bridge.state(&handle) {
            Some(LoadState::Pending) => {}
            other => panic!("Expected Some(Pending), got {:?}", other),
        }
    }
}
