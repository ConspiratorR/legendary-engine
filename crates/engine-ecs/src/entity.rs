#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Entity(u64);

impl Entity {
    const INDEX_MASK: u64 = 0x0000_00FF_FFFF_FFFF;
    const GENERATION_SHIFT: u64 = 40;

    pub fn new(index: u32, generation: u32) -> Self {
        let raw = (index as u64) | ((generation as u64) << Self::GENERATION_SHIFT);
        Self(raw)
    }

    pub fn index(self) -> u32 {
        (self.0 & Self::INDEX_MASK) as u32
    }

    pub fn generation(self) -> u32 {
        (self.0 >> Self::GENERATION_SHIFT) as u32
    }

    pub fn next_generation(self) -> Self {
        Self::new(self.index(), self.generation() + 1)
    }
}

#[cfg(test)]
mod tests {
    use super::Entity;

    #[test]
    fn test_entity_creation() {
        let e = Entity::new(0, 0);
        assert_eq!(e.index(), 0);
        assert_eq!(e.generation(), 0);
    }

    #[test]
    fn test_entity_generation_increment() {
        let e = Entity::new(0, 0);
        let e2 = e.next_generation();
        assert_eq!(e2.index(), 0);
        assert_eq!(e2.generation(), 1);
    }
}
