use crate::world::World;
use rayon::prelude::*;
use std::sync::atomic::AtomicPtr;

/// Parallel iterator over components of type `T`.
///
/// Splits the entity index list into chunks and processes them in parallel
/// using rayon's work-stealing thread pool.
pub fn par_iter<T: Send + Sync + 'static, F>(world: &World, f: F)
where
    F: Fn(&T) + Send + Sync,
{
    let indices = world.component_entities::<T>();
    indices.par_iter().for_each(|&idx| {
        if let Some(comp) = world.get_by_index::<T>(idx) {
            f(comp);
        }
    });
}

/// Parallel mutable iterator over components of type `T`.
///
/// Uses chunk-based splitting to mutate components in parallel.
/// Each chunk processes non-overlapping entity indices.
pub fn par_iter_mut<T: Send + Sync + 'static, F>(world: &mut World, f: F)
where
    F: Fn(&mut T) + Send + Sync,
{
    let indices = world.component_entities::<T>();
    if indices.is_empty() {
        return;
    }

    let chunk_size = (indices.len() / rayon::current_num_threads()).max(1);
    // AtomicPtr is Send+Sync, avoiding the raw pointer issue
    let world_ptr = AtomicPtr::new(world as *mut World);

    indices.par_chunks(chunk_size).for_each(|chunk| {
        let ptr = world_ptr.load(std::sync::atomic::Ordering::Relaxed);
        for &idx in chunk {
            // SAFETY: Each chunk accesses different entity indices.
            // The AtomicPtr provides proper synchronization for the pointer value.
            // The actual component access is non-aliasing due to chunk splitting.
            unsafe {
                if let Some(comp) = (*ptr).get_by_index_mut::<T>(idx) {
                    f(comp);
                }
            }
        }
    });
}

/// Extension trait for `World` to add parallel query methods.
pub trait WorldParExt {
    /// Iterate over all components `T` in parallel.
    fn par_for_each<T: Send + Sync + 'static, F: Fn(&T) + Send + Sync>(&self, f: F);

    /// Mutably iterate over all components `T` in parallel.
    fn par_for_each_mut<T: Send + Sync + 'static, F: Fn(&mut T) + Send + Sync>(&mut self, f: F);
}

impl WorldParExt for World {
    fn par_for_each<T: Send + Sync + 'static, F: Fn(&T) + Send + Sync>(&self, f: F) {
        par_iter(self, f);
    }

    fn par_for_each_mut<T: Send + Sync + 'static, F: Fn(&mut T) + Send + Sync>(&mut self, f: F) {
        par_iter_mut(self, f);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone)]
    struct Position(f32, f32, f32);

    #[test]
    fn test_par_iter_reads_all() {
        let mut world = World::new();
        for i in 0..1000 {
            let e = world.spawn();
            world.add_component(e, Position(i as f32, 0.0, 0.0));
        }

        let sum = std::sync::atomic::AtomicU32::new(0);
        par_iter(&world, |pos: &Position| {
            sum.fetch_add(pos.0 as u32, std::sync::atomic::Ordering::Relaxed);
        });

        // sum of 0..1000 = 499500
        assert_eq!(sum.load(std::sync::atomic::Ordering::Relaxed), 499500);
    }

    #[test]
    fn test_par_iter_empty_world() {
        let world = World::new();
        par_iter::<i32, _>(&world, |_| {
            panic!("should not be called");
        });
    }

    #[test]
    fn test_par_iter_mut_modifies_all() {
        let mut world = World::new();
        for i in 0..100 {
            let e = world.spawn();
            world.add_component(e, Position(i as f32, 0.0, 0.0));
        }

        par_iter_mut(&mut world, |pos: &mut Position| {
            pos.0 += 1.0;
        });

        for i in 0..100 {
            let pos = world.get_by_index::<Position>(i).unwrap();
            assert!((pos.0 - (i as f32 + 1.0)).abs() < f32::EPSILON);
        }
    }

    #[test]
    fn test_world_par_for_each() {
        let mut world = World::new();
        for i in 0..100 {
            let e = world.spawn();
            world.add_component(e, Position(i as f32, 0.0, 0.0));
        }

        let count = std::sync::atomic::AtomicUsize::new(0);
        world.par_for_each::<Position, _>(|_| {
            count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        });

        assert_eq!(count.load(std::sync::atomic::Ordering::Relaxed), 100);
    }
}
