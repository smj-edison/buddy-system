pub mod arena;
pub mod buddy;

pub mod pretty_print {
    use std::ops::Range;

    use generational_arena::{Arena, Index};

    use crate::{
        arena::BuddyIndexManager,
        buddy::{Node, NodeState},
    };

    #[derive(Debug)]
    pub enum PrettyState {
        Split(Box<PrettyNode>, Box<PrettyNode>),
        Available,
        Occupied,
    }

    #[derive(Debug)]
    pub struct PrettyNode {
        pub range: Range<usize>,
        pub state: PrettyState,
    }

    pub fn prettify(arena: &BuddyIndexManager, root: Index) -> PrettyNode {
        fn build(arena: &Arena<Node>, current: Index) -> PrettyNode {
            PrettyNode {
                range: arena[current].range.clone(),
                state: match arena[current].state {
                    NodeState::Available => PrettyState::Available,
                    NodeState::Occupied => PrettyState::Occupied,
                    NodeState::Split(first, second) => PrettyState::Split(
                        Box::new(build(arena, first)),
                        Box::new(build(arena, second)),
                    ),
                },
            }
        }

        build(&arena.nodes, root)
    }
}
