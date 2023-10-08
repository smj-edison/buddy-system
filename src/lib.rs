pub mod arena;
pub mod buddy;

pub mod pretty_print {
    use std::ops::Range;

    use generational_arena::{Arena, Index};

    use crate::{
        arena::BuddyBookkeeping,
        buddy::{Block, BlockState},
    };

    #[derive(Debug)]
    pub enum PrettyState {
        Split(Box<PrettyBlock>, Box<PrettyBlock>),
        Available,
        Occupied,
    }

    #[derive(Debug)]
    pub struct PrettyBlock {
        pub range: Range<usize>,
        pub state: PrettyState,
    }

    pub fn prettify(arena: &BuddyBookkeeping) -> PrettyBlock {
        fn build(arena: &Arena<Block>, current: Index) -> PrettyBlock {
            PrettyBlock {
                range: arena[current].range.clone(),
                state: match arena[current].state {
                    BlockState::Available => PrettyState::Available,
                    BlockState::Occupied => PrettyState::Occupied,
                    BlockState::Split(first, second) => PrettyState::Split(
                        Box::new(build(arena, first)),
                        Box::new(build(arena, second)),
                    ),
                },
            }
        }

        build(&arena.blocks, arena.root)
    }
}
