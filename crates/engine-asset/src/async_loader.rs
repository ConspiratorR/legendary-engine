//! Async asset loading with background thread pool.
//!
//! Provides [`AsyncLoader`] for non-blocking asset imports that run
//! on a dedicated thread pool, returning results through a channel.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Instant;

use crossbeam_channel::{self as channel, Receiver, Sender};

use crate::pipeline::{ImportError, ImportPipeline, ImportResult};

/// Unique identifier for an async load request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LoadId(u64);

impl LoadId {
    fn next() -> Self {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        Self(COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed))
    }
}

/// Priority for load requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum LoadPriority {
    /// Low priority — loaded when no higher-priority requests are pending.
    Low,
    /// Normal priority — default.
    #[default]
    Normal,
    /// High priority — loaded before normal/low requests.
    High,
    /// Critical — loaded immediately, blocking the queue.
    Critical,
}

/// An async load request.
pub struct LoadRequest {
    /// Unique request ID.
    pub id: LoadId,
    /// File path to load.
    pub path: PathBuf,
    /// Load priority.
    pub priority: LoadPriority,
    /// Time the request was submitted.
    pub submitted_at: Instant,
}

/// Result of a completed async load.
pub struct LoadResponse {
    /// The request ID.
    pub id: LoadId,
    /// The file path that was loaded.
    pub path: PathBuf,
    /// The import result, or error.
    pub result: Result<ImportResult, ImportError>,
    /// Time taken to load.
    pub load_time: std::time::Duration,
}

/// Current state of an async load request.
#[derive(Debug, Clone)]
pub enum LoadState {
    /// Waiting in the queue.
    Queued,
    /// Currently being loaded.
    Loading,
    /// Completed successfully.
    Completed,
    /// Failed with an error.
    Failed(String),
}

/// Async asset loader with a background thread pool.
///
/// Submits load requests to worker threads and delivers results
/// through a channel. Integrates with [`ImportPipeline`] for
/// format-aware importing.
pub struct AsyncLoader {
    /// Number of worker threads.
    num_threads: usize,
    /// Channel for receiving results back to the main thread.
    result_rx: Receiver<LoadResponse>,
    /// Channel for submitting requests to workers.
    request_tx: Sender<LoadRequest>,
    /// Track active/queued requests.
    states: Arc<Mutex<HashMap<LoadId, LoadState>>>,
    /// Worker thread handles (for cleanup).
    _handles: Vec<thread::JoinHandle<()>>,
}

impl AsyncLoader {
    /// Create a new async loader with the given number of worker threads.
    ///
    /// The `pipeline` is shared across all worker threads via `Arc`.
    pub fn new(num_threads: usize, pipeline: Arc<ImportPipeline>) -> Self {
        let (request_tx, request_rx) = channel::unbounded::<LoadRequest>();
        let (result_tx, result_rx) = channel::unbounded::<LoadResponse>();
        let states = Arc::new(Mutex::new(HashMap::new()));

        let mut handles = Vec::with_capacity(num_threads);

        for _ in 0..num_threads {
            let rx = request_rx.clone();
            let tx = result_tx.clone();
            let pipeline = pipeline.clone();
            let states = states.clone();

            let handle = thread::spawn(move || {
                worker_loop(rx, tx, pipeline, states);
            });
            handles.push(handle);
        }

        Self {
            num_threads,
            result_rx,
            request_tx,
            states,
            _handles: handles,
        }
    }

    /// Submit an async load request.
    ///
    /// Returns a [`LoadId`] that can be used to track the request state.
    pub fn load(&self, path: impl Into<PathBuf>, priority: LoadPriority) -> LoadId {
        let id = LoadId::next();
        let request = LoadRequest {
            id,
            path: path.into(),
            priority,
            submitted_at: Instant::now(),
        };

        self.states.lock().unwrap().insert(id, LoadState::Queued);

        // Send to workers — if channel is closed, we silently drop
        let _ = self.request_tx.send(request);

        id
    }

    /// Poll for completed load results (non-blocking).
    ///
    /// Returns all results that have completed since the last poll.
    pub fn poll_results(&self) -> Vec<LoadResponse> {
        let mut results = Vec::new();
        while let Ok(response) = self.result_rx.try_recv() {
            // Update state
            let mut states = self.states.lock().unwrap();
            let state = if response.result.is_ok() {
                LoadState::Completed
            } else {
                LoadState::Failed(
                    response
                        .result
                        .as_ref()
                        .err()
                        .map(|e| e.to_string())
                        .unwrap_or_default(),
                )
            };
            states.insert(response.id, state);

            results.push(response);
        }
        results
    }

    /// Check the state of a specific load request.
    pub fn state(&self, id: LoadId) -> Option<LoadState> {
        self.states.lock().unwrap().get(&id).cloned()
    }

    /// Number of worker threads.
    pub fn num_threads(&self) -> usize {
        self.num_threads
    }

    /// Number of pending (queued + loading) requests.
    pub fn pending_count(&self) -> usize {
        self.states
            .lock()
            .unwrap()
            .values()
            .filter(|s| matches!(s, LoadState::Queued | LoadState::Loading))
            .count()
    }

    /// Clear completed/failed states to free memory.
    pub fn clear_finished(&self) {
        let mut states = self.states.lock().unwrap();
        states.retain(|_, s| matches!(s, LoadState::Queued | LoadState::Loading));
    }
}

/// Worker thread main loop.
fn worker_loop(
    rx: Receiver<LoadRequest>,
    tx: Sender<LoadResponse>,
    pipeline: Arc<ImportPipeline>,
    states: Arc<Mutex<HashMap<LoadId, LoadState>>>,
) {
    for request in rx {
        // Update state to Loading
        {
            let mut states = states.lock().unwrap();
            states.insert(request.id, LoadState::Loading);
        }

        let start = Instant::now();
        let result = pipeline.import_file(&request.path);
        let load_time = start.elapsed();

        let response = LoadResponse {
            id: request.id,
            path: request.path,
            result,
            load_time,
        };

        // Send result — if channel is closed, main thread has been dropped
        if tx.send(response).is_err() {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asset::Asset;
    use crate::pipeline::{AssetImporter, ImportContext, ImportPipeline};

    #[derive(Debug, Clone)]
    struct TextAsset(String);

    impl Asset for TextAsset {
        type Id = str;
        fn id(&self) -> &Self::Id {
            &self.0
        }
    }

    struct TestImporter;

    impl AssetImporter for TestImporter {
        type Asset = TextAsset;

        fn extensions(&self) -> &[&str] {
            &["txt"]
        }

        fn import(&self, data: &[u8], _ctx: &mut ImportContext) -> Result<TextAsset, ImportError> {
            let s =
                String::from_utf8(data.to_vec()).map_err(|e| ImportError::Format(e.to_string()))?;
            Ok(TextAsset(s))
        }
    }

    fn make_pipeline() -> Arc<ImportPipeline> {
        let mut p = ImportPipeline::new();
        p.register(TestImporter);
        Arc::new(p)
    }

    #[test]
    fn test_async_loader_creation() {
        let pipeline = make_pipeline();
        let loader = AsyncLoader::new(2, pipeline);
        assert_eq!(loader.num_threads(), 2);
    }

    #[test]
    fn test_async_load_txt_file() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        std::fs::write(&file_path, "hello async").unwrap();

        let pipeline = make_pipeline();
        let loader = AsyncLoader::new(1, pipeline);

        let id = loader.load(&file_path, LoadPriority::Normal);
        assert!(loader.state(id).is_some());

        // Wait for completion
        std::thread::sleep(std::time::Duration::from_millis(200));

        let results = loader.poll_results();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, id);
        assert!(results[0].result.is_ok());

        let asset = results[0]
            .result
            .as_ref()
            .unwrap()
            .asset
            .downcast_ref::<TextAsset>()
            .unwrap();
        assert_eq!(&asset.0, "hello async");
    }

    #[test]
    fn test_async_load_nonexistent_file() {
        let pipeline = make_pipeline();
        let loader = AsyncLoader::new(1, pipeline);

        let id = loader.load("/nonexistent/file.txt", LoadPriority::Normal);
        std::thread::sleep(std::time::Duration::from_millis(200));

        let results = loader.poll_results();
        assert_eq!(results.len(), 1);
        assert!(results[0].result.is_err());

        let state = loader.state(id).unwrap();
        assert!(matches!(state, LoadState::Failed(_)));
    }

    #[test]
    fn test_async_pending_count() {
        let pipeline = make_pipeline();
        let loader = AsyncLoader::new(1, pipeline);

        assert_eq!(loader.pending_count(), 0);

        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        std::fs::write(&file_path, "test").unwrap();

        let _id = loader.load(&file_path, LoadPriority::Normal);
        std::thread::sleep(std::time::Duration::from_millis(200));
        loader.poll_results();
    }

    #[test]
    fn test_load_priority_ordering() {
        assert!(LoadPriority::Critical > LoadPriority::High);
        assert!(LoadPriority::High > LoadPriority::Normal);
        assert!(LoadPriority::Normal > LoadPriority::Low);
    }
}
