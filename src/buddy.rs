use std::{cmp::Ordering, ops::Range, time::Instant};

use generational_arena::{Arena, Index};

pub(crate) fn is_pow_of_two(x: usize) -> bool {
    (x != 0) && ((x & (x - 1)) == 0)
}

pub(crate) enum NodeState {
    Split(Index, Index),
    Available,
    Occupied,
}

pub(crate) struct Node {
    pub(crate) range: Range<usize>,
    pub(crate) state: NodeState,
}

/// Assumes `desired_size` is a power of 2
pub(crate) fn alloc(
    arena: &mut Arena<Node>,
    node_index: Index,
    desired_size: usize,
) -> Option<Index> {
    debug_assert!(is_pow_of_two(desired_size));

    let node = &arena[node_index];

    match node.range.len().cmp(&desired_size) {
        Ordering::Less => None,
        Ordering::Equal => {
            if let NodeState::Available = node.state {
                arena[node_index].state = NodeState::Occupied;

                Some(node_index)
            } else {
                None
            }
        }
        Ordering::Greater => match node.state {
            NodeState::Occupied => None,
            NodeState::Available => {
                let first_range = (node.range.start)..(node.range.start + node.range.len() / 2);
                let second_range = (node.range.start + node.range.len() / 2)..(node.range.end);

                let first = arena.insert(Node {
                    range: first_range,
                    state: NodeState::Available,
                });

                let second = arena.insert(Node {
                    range: second_range,
                    state: NodeState::Available,
                });

                arena[node_index].state = NodeState::Split(first, second);

                alloc(arena, first, desired_size)
            }
            NodeState::Split(first_index, second_index) => {
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

pub(crate) fn dealloc(arena: &mut Arena<Node>, node: Index) {
    arena[node].state = NodeState::Available;
}

// for my pea brain
#[repr(transparent)]
pub(crate) struct IsAvailable(bool);

pub(crate) fn tidy(arena: &mut Arena<Node>, node_index: Index) -> IsAvailable {
    // go through and merge
    let node = &arena[node_index];

    match node.state {
        NodeState::Split(first, second) => {
            let first_available = tidy(arena, first).0;
            let second_available = tidy(arena, second).0;

            if first_available && second_available {
                arena.remove(first).unwrap();
                arena.remove(second).unwrap();

                arena[node_index].state = NodeState::Available;

                IsAvailable(true)
            } else {
                IsAvailable(false)
            }
        }
        NodeState::Available => IsAvailable(true),
        NodeState::Occupied => IsAvailable(false),
    }
}

pub(crate) fn tidy_gas(arena: &mut Arena<Node>, node_index: Index, gas: usize) -> IsAvailable {
    if gas == 0 {
        return IsAvailable(false);
    }

    // go through and merge
    let node = &arena[node_index];

    match node.state {
        NodeState::Split(first, second) => {
            let first_available = tidy_gas(arena, first, gas - 1).0;
            let second_available = tidy_gas(arena, second, gas - 1).0;

            if first_available && second_available {
                arena.remove(first).unwrap();
                arena.remove(second).unwrap();

                arena[node_index].state = NodeState::Available;

                IsAvailable(true)
            } else {
                IsAvailable(false)
            }
        }
        NodeState::Available => IsAvailable(true),
        NodeState::Occupied => IsAvailable(false),
    }
}

pub(crate) fn tidy_timed(
    arena: &mut Arena<Node>,
    node_index: Index,
    deadline: Instant,
) -> IsAvailable {
    if Instant::now() >= deadline {
        // say it isn't available so the recursion chain stops
        return IsAvailable(false);
    }

    // go through and merge
    let node = &arena[node_index];

    match node.state {
        NodeState::Split(first, second) => {
            let first_available = tidy_timed(arena, first, deadline).0;
            let second_available = tidy_timed(arena, second, deadline).0;

            if first_available && second_available {
                arena.remove(first).unwrap();
                arena.remove(second).unwrap();

                arena[node_index].state = NodeState::Available;

                IsAvailable(true)
            } else {
                IsAvailable(false)
            }
        }
        NodeState::Available => IsAvailable(true),
        NodeState::Occupied => IsAvailable(false),
    }
}
