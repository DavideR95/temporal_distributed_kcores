use bitvec::prelude::*;
use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;

pub struct DynamicNode<T, F>
where
    T: Clone + Copy + Into<usize> + std::fmt::Display + Ord + std::convert::From<usize> + Hash,
    F: Fn(&BitVec) -> bool + std::clone::Clone,
{
    // Node-specific attributes
    id: Option<T>,                 // Node id, if None the node is not present
    neighbors: HashMap<T, BitVec>, // Keeps track of the last t snapshots of neighbors
    memory_size: usize,
    // Algorithm Variables
    changed: bool,
    coreness: usize,
    count: Vec<usize>,
    estimate: HashMap<T, (usize, usize)>, // First value: estimate, second: timestamp
    activation_function: F,
    time: usize,
    activated: bool,
    dirty: bool,
    paused: bool,
    last_sent: usize,
}

impl<T, F> Clone for AliveNode<T, F>
where
    T: Clone + Copy + Into<usize> + std::fmt::Display + Ord + std::convert::From<usize> + Hash,
    F: Fn(&BitVec) -> bool + Clone,
{
    fn clone(&self) -> Self {
        AliveNode {
            id: self.id,
            neighbors: self.neighbors.clone(),
            memory_size: self.memory_size,
            changed: self.changed,
            coreness: self.coreness,
            count: self.count.clone(),
            estimate: self.estimate.clone(),
            activation_function: self.activation_function.clone(),
            time: self.time,
            activated: self.activated,
            dirty: self.dirty,
            paused: self.paused,
            last_sent: self.last_sent,
        }
    }
}

impl<T, F> std::fmt::Debug for DynamicNode<T, F>
where
    T: Clone
        + Debug
        + Copy
        + Into<usize>
        + std::fmt::Display
        + Ord
        + std::convert::From<usize>
        + Hash,
    F: Fn(&BitVec) -> bool + std::clone::Clone,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Specify what you want to print here
        if self.id != None {
            write!(
                f,
                "[id: {}, neighs: {:?}]",
                self.id.unwrap(),
                self.neighbors
            )
        } else {
            write!(f, "")
        }
    }
}
