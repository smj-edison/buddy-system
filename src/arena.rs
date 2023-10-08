use std::{iter::repeat_with, ops::Range, sync::mpsc, time::Instant};

use generational_arena::{Arena, Index};

use crate::{
    buddy::{self, is_pow_of_two, Block, BlockState},
    pretty_print::prettify,
};

/// NOT Copy or Clone, to make sure it's unique
#[derive(Debug)]
pub struct Allocation {
    index: Index,
    range: Range<usize>,
    to_remove: mpsc::Sender<Index>,
}

impl Allocation {
    pub fn range(&self) -> Range<usize> {
        self.range.clone()
    }
}

impl Drop for Allocation {
    fn drop(&mut self) {
        // don't panic here; leaking is not considered unsafe if the
        // receiver doesn't get it for whatever reason
        let _ = self.to_remove.send(self.index);
    }
}

pub struct BuddyBookkeeping {
    pub(crate) blocks: Arena<Block>,
    pub(crate) root: Index,
    to_remove_sender: mpsc::Sender<Index>,
    to_remove_receiver: mpsc::Receiver<Index>,
    min_block_size: usize,
    max_block_size: usize,
}

impl BuddyBookkeeping {
    pub fn new(size: usize, min_block_size: usize, max_block_size: usize) -> BuddyBookkeeping {
        assert!(is_pow_of_two(size));
        assert!(max_block_size <= size);
        assert!(min_block_size <= max_block_size);

        let mut new_arena = Arena::new();
        let root = new_arena.insert(Block {
            range: 0..size,
            state: BlockState::Available,
        });

        let (sender, receiver) = mpsc::channel();

        BuddyBookkeeping {
            to_remove_sender: sender,
            to_remove_receiver: receiver,
            blocks: new_arena,
            root: root,
            min_block_size,
            max_block_size,
        }
    }

    pub fn alloc(&mut self, count: usize) -> Option<Allocation> {
        let best_size = if 2_u32.pow(count.ilog2()) as usize == count {
            count
        } else {
            2_u32.pow(count.ilog2() + 1) as usize
        }
        .max(self.min_block_size)
        .min(self.max_block_size);

        if best_size < count {
            return None;
        }

        buddy::alloc(&mut self.blocks, self.root, best_size).map(|x| Allocation {
            index: x,
            range: (self.blocks[x].range.start)..(self.blocks[x].range.start + count),
            to_remove: self.to_remove_sender.clone(),
        })
    }

    pub fn tidy(&mut self) {
        while let Ok(index) = self.to_remove_receiver.try_recv() {
            buddy::dealloc(&mut self.blocks, index);
        }

        buddy::tidy(&mut self.blocks, self.root);
    }

    pub fn tidy_gas(&mut self, gas: usize) {
        let mut gas = gas;

        while let Ok(index) = self.to_remove_receiver.try_recv() {
            if gas == 0 {
                return;
            }

            buddy::dealloc(&mut self.blocks, index);

            gas -= 1;
        }

        buddy::tidy_gas(&mut self.blocks, self.root, &mut gas);
    }

    pub fn tidy_timed(&mut self, deadline: Instant) {
        while let Ok(index) = self.to_remove_receiver.try_recv() {
            if Instant::now() >= deadline {
                return;
            }

            buddy::dealloc(&mut self.blocks, index);
        }

        buddy::tidy_timed(&mut self.blocks, self.root, deadline);
    }
}

pub struct BuddyArena<T> {
    elements: Box<[T]>,
    bookkeeping: BuddyBookkeeping,
}

impl<T> BuddyArena<T> {
    pub fn new(size: usize, min_block_size: usize, max_block_size: usize) -> BuddyArena<T>
    where
        T: Default,
    {
        let elements_vec: Vec<T> = repeat_with(|| T::default()).take(size).collect();

        BuddyArena {
            elements: elements_vec.into(),
            bookkeeping: BuddyBookkeeping::new(size, min_block_size, max_block_size),
        }
    }

    pub fn bookkeeping(&self) -> &BuddyBookkeeping {
        &self.bookkeeping
    }

    pub fn view(&self, a: &Allocation) -> &[T] {
        &self.elements[a.range()]
    }

    pub fn view_mut(&mut self, a: &Allocation) -> &mut [T] {
        &mut self.elements[a.range()]
    }

    pub fn alloc(&mut self, count: usize) -> Option<Allocation> {
        self.bookkeeping.alloc(count)
    }

    pub fn tidy(&mut self) {
        self.bookkeeping.tidy();
    }

    pub fn tidy_gas(&mut self, gas: usize) {
        self.bookkeeping.tidy_gas(gas);
    }

    pub fn tidy_timed(&mut self, deadline: Instant) {
        self.bookkeeping.tidy_timed(deadline);
    }
}

#[test]
fn test() {
    let mut arena: BuddyArena<u8> = BuddyArena::new(2048, 8, 256);

    let a1 = arena.alloc(64).unwrap();
    let a2 = arena.alloc(24).unwrap();
    let a3 = arena.alloc(2).unwrap();
    let a4 = arena.alloc(7).unwrap();
    let a5 = arena.alloc(31).unwrap();
    let a6 = arena.alloc(60).unwrap();

    println!("{:#?}", prettify(arena.bookkeeping()));

    let string = "foobar";

    let a = arena.alloc(string.bytes().len()).unwrap();
    let view = arena.view_mut(&a);

    view.copy_from_slice(string.as_bytes());

    let str_view = std::str::from_utf8(view).unwrap();

    dbg!(str_view);
}
