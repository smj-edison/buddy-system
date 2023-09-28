use std::{iter::repeat_with, ops::Range, sync::mpsc, time::Instant};

use generational_arena::{Arena, Index};

use crate::buddy::{self, is_pow_of_two, Node, NodeState};

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

pub struct BuddyIndexManager {
    pub(crate) nodes: Arena<Node>,
    root: Index,
    to_remove_sender: mpsc::Sender<Index>,
    to_remove_receiver: mpsc::Receiver<Index>,
}

impl BuddyIndexManager {
    pub fn new(size: usize) -> BuddyIndexManager {
        assert!(is_pow_of_two(size));

        let mut new_arena = Arena::new();
        let root = new_arena.insert(Node {
            range: 0..size,
            state: NodeState::Available,
        });

        let (sender, receiver) = mpsc::channel();

        BuddyIndexManager {
            to_remove_sender: sender,
            to_remove_receiver: receiver,
            nodes: new_arena,
            root: root,
        }
    }

    pub fn alloc(&mut self, count: usize) -> Option<Allocation> {
        let best_size = if 2_u32.pow(count.ilog2()) as usize == count {
            count
        } else {
            2_u32.pow(count.ilog2() + 1) as usize
        };

        println!("best size: {}", best_size);

        buddy::alloc(&mut self.nodes, self.root, best_size).map(|x| Allocation {
            index: x,
            range: (self.nodes[x].range.start)..(self.nodes[x].range.start + count),
            to_remove: self.to_remove_sender.clone(),
        })
    }

    pub fn tidy(&mut self) {
        while let Ok(index) = self.to_remove_receiver.try_recv() {
            buddy::dealloc(&mut self.nodes, index);
        }

        buddy::tidy(&mut self.nodes, self.root);
    }

    pub fn tidy_gas(&mut self, mut gas: usize) {
        while let Ok(index) = self.to_remove_receiver.try_recv() {
            if gas == 0 {
                return;
            }

            buddy::dealloc(&mut self.nodes, index);

            gas -= 1;
        }

        buddy::tidy_gas(&mut self.nodes, self.root, gas);
    }

    pub fn tidy_timed(&mut self, deadline: Instant) {
        while let Ok(index) = self.to_remove_receiver.try_recv() {
            if Instant::now() >= deadline {
                return;
            }

            buddy::dealloc(&mut self.nodes, index);
        }

        buddy::tidy_timed(&mut self.nodes, self.root, deadline);
    }
}

pub struct BuddyArena<T, const N: usize> {
    elements: [T; N],
    manager: BuddyIndexManager,
}

impl<T, const N: usize> BuddyArena<T, N> {
    pub fn new() -> BuddyArena<T, N>
    where
        T: Default,
    {
        let elements_vec: Vec<T> = repeat_with(|| T::default()).take(N).collect();
        let elements: [T; N] = match elements_vec.try_into() {
            Ok(elements) => elements,
            Err(_) => unreachable!(),
        };

        BuddyArena {
            elements,
            manager: BuddyIndexManager::new(N),
        }
    }

    pub fn view(&self, a: &Allocation) -> &[T] {
        &self.elements[a.range()]
    }

    pub fn view_mut(&mut self, a: &Allocation) -> &mut [T] {
        &mut self.elements[a.range()]
    }

    pub fn alloc(&mut self, count: usize) -> Option<Allocation> {
        self.manager.alloc(count)
    }

    pub fn tidy(&mut self) {
        self.manager.tidy();
    }

    pub fn tidy_gas(&mut self, gas: usize) {
        self.manager.tidy_gas(gas);
    }

    pub fn tidy_timed(&mut self, deadline: Instant) {
        self.manager.tidy_timed(deadline);
    }
}

#[test]
fn test() {
    let mut arena: BuddyArena<u8, 512> = BuddyArena::new();

    let string = "foobar";

    let a = arena.alloc(string.bytes().len()).unwrap();
    let view = arena.view_mut(&a);

    view.copy_from_slice(string.as_bytes());

    let str_view = std::str::from_utf8(view).unwrap();

    dbg!(str_view);
}
