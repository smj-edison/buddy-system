//! Buddy system implementations.
//!
//! Note: everything in this module is unchecked. It shouldn't panic (the only `unwraps`
//! should be unreachable unless an internal invariant is broken), but it won't behave as
//! expected if it's given the wrong inputs or state.

use std::{cmp::Ordering, ops::Range, time::Instant};

use generational_arena::{Arena, Index};

pub(crate) fn is_pow_of_two(x: usize) -> bool {
    (x != 0) && ((x & (x - 1)) == 0)
}

pub(crate) enum BlockState {
    Split(Index, Index),
    Available,
    Occupied,
}

pub struct Block {
    pub(crate) range: Range<usize>,
    pub(crate) state: BlockState,
}

/// Assumes `desired_size` is a power of 2
pub fn alloc(arena: &mut Arena<Block>, block_index: Index, desired_size: usize) -> Option<Index> {
    debug_assert!(is_pow_of_two(desired_size));

    let block = &arena[block_index];

    match block.range.len().cmp(&desired_size) {
        Ordering::Less => None,
        Ordering::Equal => {
            if let BlockState::Available = block.state {
                arena[block_index].state = BlockState::Occupied;

                Some(block_index)
            } else {
                None
            }
        }
        Ordering::Greater => match block.state {
            BlockState::Occupied => None,
            BlockState::Available => {
                let first_range = (block.range.start)..(block.range.start + block.range.len() / 2);
                let second_range = (block.range.start + block.range.len() / 2)..(block.range.end);

                let first = arena.insert(Block {
                    range: first_range,
                    state: BlockState::Available,
                });

                let second = arena.insert(Block {
                    range: second_range,
                    state: BlockState::Available,
                });

                arena[block_index].state = BlockState::Split(first, second);

                alloc(arena, first, desired_size)
            }
            BlockState::Split(first_index, second_index) => {
                if let Some(result) = alloc(arena, first_index, desired_size) {
                    Some(result)
                } else if let Some(result) = alloc(arena, second_index, desired_size) {
                    Some(result)
                } else {
                    None
                }
            }
        },
    }
}

pub(crate) fn dealloc(arena: &mut Arena<Block>, block_index: Index) {
    arena[block_index].state = BlockState::Available;
}

#[repr(transparent)]
pub struct IsAvailable(bool);

pub fn tidy(arena: &mut Arena<Block>, block_index: Index) -> IsAvailable {
    // go through and merge
    let block = &arena[block_index];

    match block.state {
        BlockState::Split(first, second) => {
            let first_available = tidy(arena, first).0;
            let second_available = tidy(arena, second).0;

            if first_available && second_available {
                arena.remove(first).unwrap();
                arena.remove(second).unwrap();

                arena[block_index].state = BlockState::Available;

                IsAvailable(true)
            } else {
                IsAvailable(false)
            }
        }
        BlockState::Available => IsAvailable(true),
        BlockState::Occupied => IsAvailable(false),
    }
}

pub fn tidy_gas(arena: &mut Arena<Block>, block_index: Index, gas: &mut usize) -> IsAvailable {
    if *gas == 0 {
        return IsAvailable(false);
    }

    *gas -= 1;

    // go through and merge
    let block = &arena[block_index];

    match block.state {
        BlockState::Split(first, second) => {
            let first_available = tidy_gas(arena, first, gas).0;
            let second_available = tidy_gas(arena, second, gas).0;

            if first_available && second_available {
                arena.remove(first).unwrap();
                arena.remove(second).unwrap();

                arena[block_index].state = BlockState::Available;

                IsAvailable(true)
            } else {
                IsAvailable(false)
            }
        }
        BlockState::Available => IsAvailable(true),
        BlockState::Occupied => IsAvailable(false),
    }
}

pub fn tidy_timed(arena: &mut Arena<Block>, block_index: Index, deadline: Instant) -> IsAvailable {
    if Instant::now() >= deadline {
        // return not available so the recursion chain stops
        return IsAvailable(false);
    }

    // go through and merge
    let block = &arena[block_index];

    match block.state {
        BlockState::Split(first, second) => {
            let first_available = tidy_timed(arena, first, deadline).0;
            let second_available = tidy_timed(arena, second, deadline).0;

            if first_available && second_available {
                arena.remove(first).unwrap();
                arena.remove(second).unwrap();

                arena[block_index].state = BlockState::Available;

                IsAvailable(true)
            } else {
                IsAvailable(false)
            }
        }
        BlockState::Available => IsAvailable(true),
        BlockState::Occupied => IsAvailable(false),
    }
}
