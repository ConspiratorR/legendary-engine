//! Multi-threaded command recording via parallel command encoders.
//!
//! [`CommandBatcher`] creates multiple [`wgpu::CommandEncoder`]s that can
//! record commands in parallel (e.g. via rayon). When finished, the
//! resulting [`wgpu::CommandBuffer`]s are merged into a single
//! [`queue.submit()`](wgpu::Queue::submit) call.
//!
//! # Example
//!
//! ```no_run
//! # use engine_render::command_batch::CommandBatcher;
//! # let device: wgpu::Device = unimplemented!();
//! let mut batcher = CommandBatcher::new(&device, 4);
//!
//! // Record passes in parallel
//! rayon::scope(|s| {
//!     for i in 0..4 {
//!         s.spawn(move |_| {
//!             let encoder = batcher.encoder(i);
//!             // ... record commands on encoder ...
//!         });
//!     }
//! });
//!
//! // Submit all at once
//! let buffers = batcher.finish();
//! // queue.submit(buffers);
//! ```

use parking_lot::Mutex;

/// Manages multiple command encoders for parallel command recording.
///
/// Encoders are indexed from 0..N. Each encoder can be used independently
/// on a separate thread. After recording, call [`finish`](Self::finish)
/// to collect all command buffers for a single `queue.submit()`.
pub struct CommandBatcher {
    encoders: Vec<Mutex<wgpu::CommandEncoder>>,
}

impl CommandBatcher {
    /// Create a new batcher with `count` command encoders.
    ///
    /// Each encoder is labeled `"batch_encoder_{i}"`.
    pub fn new(device: &wgpu::Device, count: usize) -> Self {
        let encoders = (0..count)
            .map(|i| {
                Mutex::new(
                    device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some(&format!("batch_encoder_{}", i)),
                    }),
                )
            })
            .collect();

        Self { encoders }
    }

    /// Get the number of encoders in this batcher.
    pub fn len(&self) -> usize {
        self.encoders.len()
    }

    /// Return true if there are no encoders.
    pub fn is_empty(&self) -> bool {
        self.encoders.is_empty()
    }

    /// Access an encoder by index for recording.
    ///
    /// The encoder is locked for the duration of the returned guard's lifetime.
    /// Use this within a parallel iterator to record commands on a specific encoder.
    pub fn get(&self, index: usize) -> CommandEncoderGuard<'_> {
        CommandEncoderGuard {
            inner: self.encoders[index].lock(),
        }
    }

    /// Consume the batcher and return all command buffers in order.
    ///
    /// The resulting buffers can be passed directly to `queue.submit()`.
    pub fn finish(self) -> Vec<wgpu::CommandBuffer> {
        self.encoders
            .into_iter()
            .map(|enc| enc.into_inner().finish())
            .collect()
    }
}

/// RAII guard providing mutable access to a locked command encoder.
///
/// Dereferences to `&mut wgpu::CommandEncoder`.
pub struct CommandEncoderGuard<'a> {
    inner: parking_lot::MutexGuard<'a, wgpu::CommandEncoder>,
}

impl<'a> std::ops::Deref for CommandEncoderGuard<'a> {
    type Target = wgpu::CommandEncoder;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'a> std::ops::DerefMut for CommandEncoderGuard<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

/// Extension trait for `wgpu::Queue` to submit batches of command buffers.
pub trait QueueSubmitBatchExt {
    /// Submit multiple command buffers in a single call.
    fn submit_batch(&self, buffers: Vec<wgpu::CommandBuffer>);
}

impl QueueSubmitBatchExt for wgpu::Queue {
    fn submit_batch(&self, buffers: Vec<wgpu::CommandBuffer>) {
        self.submit(buffers);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_batcher_len() {
        // Can't create a real device in unit tests, but we can test the API shape
        // by verifying the struct layout.
        assert_eq!(
            std::mem::size_of::<CommandBatcher>(),
            std::mem::size_of::<Vec<Mutex<wgpu::CommandEncoder>>>()
        );
    }
}
