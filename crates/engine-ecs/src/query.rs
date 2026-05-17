use crate::world::World;
use std::marker::PhantomData;

pub struct Query<T> {
    _marker: PhantomData<T>,
}

impl<A: 'static> Default for Query<A> {
    fn default() -> Self {
        Self::new()
    }
}

impl<A: 'static> Query<A> {
    pub fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }

    pub fn iter<'a>(&self, world: &'a World) -> impl Iterator<Item = &'a A> {
        let indices: Vec<_> = world.component_entities::<A>();
        indices
            .into_iter()
            .filter_map(move |idx| world.get_by_index::<A>(idx))
    }

    pub fn iter_mut<'a>(&self, world: &'a mut World) -> QueryIterMut<'a, A> {
        let indices = world.component_entities::<A>();
        QueryIterMut {
            indices,
            index: 0,
            world: world as *mut World,
            _marker: PhantomData,
        }
    }
}

pub struct QueryPair<A, B> {
    _marker: PhantomData<(A, B)>,
}

impl<A: 'static, B: 'static> Default for QueryPair<A, B> {
    fn default() -> Self {
        Self::new()
    }
}

impl<A: 'static, B: 'static> QueryPair<A, B> {
    pub fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }

    pub fn iter<'a>(&self, world: &'a World) -> impl Iterator<Item = (&'a A, &'a B)> {
        let valid: Vec<_> = {
            let ents_a = world.component_entities::<A>();
            let ents_b = world.component_entities::<B>();
            ents_a
                .iter()
                .filter(|idx| ents_b.contains(idx))
                .copied()
                .collect()
        };
        valid.into_iter().filter_map(move |idx| {
            Some((world.get_by_index::<A>(idx)?, world.get_by_index::<B>(idx)?))
        })
    }

    pub fn iter_mut<'a>(&self, world: &'a mut World) -> QueryPairIterMut<'a, A, B> {
        let valid: Vec<_> = {
            let ents_a = world.component_entities::<A>();
            let ents_b = world.component_entities::<B>();
            ents_a
                .iter()
                .filter(|idx| ents_b.contains(idx))
                .copied()
                .collect()
        };
        QueryPairIterMut {
            indices: valid,
            index: 0,
            world: world as *mut World,
            _marker: PhantomData,
        }
    }
}

pub struct QueryPairIterMut<'a, A, B> {
    indices: Vec<u32>,
    index: usize,
    world: *mut World,
    _marker: PhantomData<(&'a mut A, &'a mut B)>,
}

impl<'a, A: 'static, B: 'static> Iterator for QueryPairIterMut<'a, A, B> {
    type Item = (&'a mut A, &'a mut B);

    fn next(&mut self) -> Option<Self::Item> {
        let idx = self.index;
        self.index += 1;
        if idx < self.indices.len() {
            let entity_idx = self.indices[idx];
            unsafe {
                let a = (*self.world).get_by_index_mut::<A>(entity_idx)?;
                let b = (*self.world).get_by_index_mut::<B>(entity_idx)?;
                Some((a, b))
            }
        } else {
            None
        }
    }
}

pub struct QueryIterMut<'a, A> {
    indices: Vec<u32>,
    index: usize,
    world: *mut World,
    _marker: PhantomData<&'a mut A>,
}

impl<'a, A: 'static> Iterator for QueryIterMut<'a, A> {
    type Item = &'a mut A;

    fn next(&mut self) -> Option<Self::Item> {
        let idx = self.index;
        self.index += 1;
        if idx < self.indices.len() {
            let entity_idx = self.indices[idx];
            unsafe { (*self.world).get_by_index_mut::<A>(entity_idx) }
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{query::Query, query::QueryPair, world::World};

    struct Pos(f32, f32);
    struct Vel(f32, f32);

    #[test]
    fn test_query_single_component_iter() {
        let mut world = World::new();
        let e1 = world.spawn();
        world.add_component(e1, Pos(0.0, 0.0));
        let e2 = world.spawn();
        world.add_component(e2, Pos(1.0, 1.0));

        let query = Query::<Pos>::new();
        let results: Vec<&Pos> = query.iter(&world).collect();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_query_two_component_iter() {
        let mut world = World::new();
        let e1 = world.spawn();
        world.add_component(e1, Pos(0.0, 0.0));
        world.add_component(e1, Vel(1.0, 0.0));
        let e2 = world.spawn();
        world.add_component(e2, Pos(1.0, 1.0));
        world.add_component(e2, Vel(0.0, 1.0));
        let e3 = world.spawn();
        world.add_component(e3, Pos(2.0, 2.0));

        let query = QueryPair::<Pos, Vel>::new();
        let results: Vec<(&Pos, &Vel)> = query.iter(&world).collect();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0.0, 0.0);
        assert_eq!(results[1].0.0, 1.0);
    }

    #[test]
    fn test_query_iter_mut() {
        let mut world = World::new();
        let e = world.spawn();
        world.add_component(e, Pos(0.0, 0.0));
        world.add_component(e, Vel(1.0, 0.0));

        let query = Query::<Pos>::new();
        for pos in query.iter_mut(&mut world) {
            pos.0 += 1.0;
        }

        let pos = world.get::<Pos>(e).unwrap();
        assert_eq!(pos.0, 1.0);
    }

    #[test]
    fn test_query_no_match() {
        let mut world = World::new();
        let e = world.spawn();
        world.add_component(e, Vel(1.0, 0.0));

        let query = Query::<Pos>::new();
        let results: Vec<&Pos> = query.iter(&world).collect();
        assert!(results.is_empty());
    }
}
