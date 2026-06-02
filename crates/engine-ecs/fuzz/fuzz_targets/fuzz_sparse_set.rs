#![no_main]
use libfuzzer_sys::fuzz_target;
use arbitrary::Arbitrary;
use engine_ecs::component::SparseSet;

#[derive(Arbitrary, Debug)]
enum SparseSetOp {
    Insert { index: u32, value: f32 },
    Remove { index: u32 },
    Get { index: u32 },
    GetMut { index: u32 },
    Compact,
    Len,
    IsEmpty,
    Entities,
    SparseLen,
    WastedSlots,
}

fuzz_target!(|ops: Vec<SparseSetOp>| {
    let mut set = SparseSet::<f32>::new();

    for op in ops {
        match op {
            SparseSetOp::Insert { index, value } => {
                // Cap index to prevent excessive memory allocation
                let idx = index % 10_000;
                set.insert(idx, value);
            }
            SparseSetOp::Remove { index } => {
                let idx = index % 10_000;
                let _ = set.remove(idx);
            }
            SparseSetOp::Get { index } => {
                let idx = index % 10_000;
                let _ = set.get(idx);
            }
            SparseSetOp::GetMut { index } => {
                let idx = index % 10_000;
                let _ = set.get_mut(idx);
            }
            SparseSetOp::Compact => {
                set.compact();
            }
            SparseSetOp::Len => {
                let _ = set.len();
            }
            SparseSetOp::IsEmpty => {
                let _ = set.is_empty();
            }
            SparseSetOp::Entities => {
                let _ = set.entities();
            }
            SparseSetOp::SparseLen => {
                let _ = set.sparse_len();
            }
            SparseSetOp::WastedSlots => {
                let _ = set.wasted_slots();
            }
        }
    }

    // Verify invariants
    assert!(set.len() <= set.sparse_len());
    assert_eq!(set.is_empty(), set.entities().is_empty());
    assert_eq!(set.entities().len(), set.len());
});
