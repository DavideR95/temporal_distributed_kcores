#![allow(dead_code)]
use bitvec::prelude::*;
use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;

#[derive(Debug)]
pub struct Message<T> {
    mtype: u8, // 0 = coreness update, 1 = degree update, 2 = answer
    from: T,
    to: T,
    core: Option<usize>,
    degree: Option<usize>,
    reset: bool,
    time: usize,
    broadcast: bool,
}

impl<T> Message<T> {
    pub fn build_coreness_msg(_from: T, _to: T, _core: usize, _t: usize) -> Self {
        Message {
            mtype: 0,
            from: _from,
            to: _to,
            core: Some(_core),
            degree: None,
            reset: false,
            time: _t,
            broadcast: false,
        }
    }

    pub fn build_degree_msg(_from: T, _to: T, _deg: usize, _t: usize) -> Self {
        Message {
            mtype: 1,
            from: _from,
            to: _to,
            core: None,
            degree: Some(_deg),
            reset: false,
            time: _t,
            broadcast: true,
        }
    }

    pub fn build_answer(_from: T, _to: T, _core: usize, _t: usize) -> Self {
        Message {
            mtype: 2,
            from: _from,
            to: _to,
            core: Some(_core),
            degree: None,
            reset: false,
            time: _t,
            broadcast: true,
        }
    }

    pub fn set_reset(&mut self) {
        self.reset = true;
    }

    pub fn is_reset(&self) -> bool {
        self.reset
    }

    pub fn set_broadcast(&mut self) {
        self.broadcast = true;
    }

    pub fn unset_broadcast(&mut self) {
        self.broadcast = false;
    }
}

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

impl<T, F> Clone for DynamicNode<T, F>
where
    T: Clone + Copy + Into<usize> + std::fmt::Display + Ord + std::convert::From<usize> + Hash,
    F: Fn(&BitVec) -> bool + Clone,
{
    fn clone(&self) -> Self {
        DynamicNode {
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

impl<T, F> DynamicNode<T, F>
// Adesso il nodo è blind rispetto agli intervalli
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
    pub fn new(nid: Option<T>, _memory_size: usize, af: F) -> Self {
        DynamicNode {
            id: nid,
            neighbors: HashMap::new(),
            changed: false,
            coreness: usize::MAX,
            estimate: HashMap::new(),
            count: vec![],
            memory_size: _memory_size,
            activation_function: af,
            time: 0,
            activated: false,
            paused: false,
            dirty: false,
            last_sent: 0,
        }
    }

    pub fn get_id(&self) -> Option<T> {
        self.id
    }

    pub fn set_id(&mut self, id: T) {
        self.id = Some(id);
    }

    pub fn change_activation_function(&mut self, f: F) {
        self.activation_function = f;
    }

    pub fn deg(&self) -> usize {
        self.neighs().len()
    }

    pub fn get_coreness(&self) -> usize {
        if self.get_id().is_none() {
            return usize::MAX;
        }
        self.coreness
    }

    pub fn neighs(&self) -> Vec<T> {
        self.neighbors
            .keys()
            .cloned()
            .filter(|key| {
                // Get only the last self.memory_size elem
                (self.activation_function)(
                    &self.neighbors[key]
                        [self.neighbors[key].len().saturating_sub(self.memory_size)..]
                        .to_bitvec(),
                )
            })
            .collect()
    }

    pub fn advance_time(&mut self) {
        self.time += 1;
        for neigh in self.neighbors.values_mut() {
            neigh.push(false);
        }
    }

    pub fn add_neighbor_latest_time(&mut self, neighbor: T) {
        self.neighbors
            .entry(neighbor)
            .and_modify(|bitv| {
                bitv.last_mut().unwrap().set(true);
                if (self.activation_function)(
                    &bitv[bitv.len().saturating_sub(self.memory_size)..].to_bitvec(),
                ) {
                    self.estimate
                        .entry(neighbor) // se c'è già non faccio niente
                        .or_insert_with(|| {
                            self.dirty = true;
                            (usize::MAX, self.time)
                        }); // Altrimenti lo inserisco nuovo nell'estimate
                }
            })
            .or_insert_with(|| {
                let mut bitv = bitvec![0; self.time];
                bitv.last_mut().unwrap().set(true);
                if (self.activation_function)(
                    &bitv[bitv.len().saturating_sub(self.memory_size)..].to_bitvec(),
                ) {
                    self.estimate
                        .entry(neighbor) // se c'è già non faccio niente
                        .or_insert_with(|| {
                            self.dirty = true;
                            (usize::MAX, self.time)
                        }); // Altrimenti lo inserisco nuovo nell'estimate
                }
                bitv
            });
    }

    pub fn reset(&mut self) {}

    pub fn needs_to_send(&self) -> bool {
        if self.id.is_some() {
            return self.changed;
        }
        false
    }

    pub fn send_msg(&mut self) -> Option<Message<T>> {
        if self.id.is_none() {
            return None;
        }

        if self.changed {
            self.changed = false;
            self.activated = true;
            self.last_sent = self.time;
            let mut to_send =
                Message::build_coreness_msg(self.id.unwrap(), 0.into(), self.coreness, self.time);

            to_send.set_broadcast();

            return Some(to_send);
        }

        None
    }

    pub fn receive_msg(&mut self, msg: &Message<T>) -> Option<Message<T>> {
        let mut overwrite = false;

        match self.estimate.get(&msg.from) {
            Some(&est) => {
                if (msg.core.unwrap() < est.0 && est.1 == msg.time)
                    || (msg.core.unwrap() > est.0 && est.1 < msg.time)
                    || msg.time > est.1
                {
                    overwrite = true;
                }
            }
            None => overwrite = true,
        }

        if overwrite {
            self.estimate
                .insert(msg.from, (msg.core.unwrap(), msg.time));
            let t = self.compute_coreness();
            if t != self.coreness {
                self.coreness = t;
                self.changed = true;
                self.activated = true;
            }
        }

        None
    }

    pub fn compute_coreness(&mut self) -> usize {
        let mut count = vec![0; self.estimate.len() + 1];

        for &core in self.estimate.values() {
            if core.0 < usize::MAX {
                if core.0 < count.len() {
                    count[core.0] += 1;
                } else {
                    count[self.estimate.len()] += 1;
                }
            } else {
                return std::cmp::min(self.coreness, self.deg());
            }
        }

        let mut total = 0;
        for i in (0..count.len()).rev() {
            total += count[i];
            if total >= i {
                if i == 0 {
                    return usize::MAX;
                }
                return i;
            }
        }

        self.deg()
    }
}

#[derive(Debug)]
pub struct DynamicGraph<T, F>
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
    nodes: Vec<DynamicNode<T, F>>,
    edges: usize,
    memory_size: usize,
    activation_function: F,
}

impl<T, F> DynamicGraph<T, F>
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
    pub fn new(activation_function: F) -> Self {
        DynamicGraph {
            nodes: vec![],
            edges: 0,
            memory_size: 5,
            activation_function,
        }
    }

    pub fn from_n_and_mem_size(nid: usize, mem_size: usize, _activation_function: F) -> Self {
        DynamicGraph {
            nodes: vec![DynamicNode::new(None, mem_size, _activation_function.clone()); nid],
            edges: 0,
            memory_size: mem_size,
            activation_function: _activation_function,
        }
    }

    pub fn get_n(&self) -> usize {
        let mut count = 0;
        self.nodes.iter().for_each(|n| {
            if n.get_id().is_some() {
                count += 1;
            }
        });
        count
    }

    pub fn get_m(&self) -> usize {
        self.edges
    }

    pub fn set_m(&mut self, _edges: usize) {
        self.edges = _edges;
    }

    pub fn get_node(&self, id: usize) -> &DynamicNode<T, F> {
        &self.nodes[id]
    }

    pub fn get_neighs(&self, who: usize) -> Vec<T> {
        let mut ans = self.nodes[who].neighs();

        ans.sort_unstable();

        ans
    }

    pub fn new_snapshot(&mut self, list: &Vec<(T, T)>) {
        // Advance the timestamp for all nodes
        self.nodes.iter_mut().for_each(|v| v.advance_time());

        for &elem in list {
            assert_ne!(elem.0, elem.1);
            let u: usize = elem.0.into();
            let v: usize = elem.1.into();

            if self.nodes[u].get_id().is_none() {
                self.nodes[u].set_id(elem.0);
            }
            if self.nodes[v].get_id().is_none() {
                self.nodes[v].set_id(elem.1);
            }

            self.nodes[u].add_neighbor_latest_time(elem.1);
            self.nodes[v].add_neighbor_latest_time(elem.0);
            self.edges += 1;
        }

        self.nodes.iter_mut().for_each(|v| v.reset());
    }

    pub fn run(&mut self) -> usize {
        let mut go_on = false;
        let mut node_queue: Vec<usize> = Vec::with_capacity(self.get_n());

        self.nodes.iter().for_each(|node| {
            if node.needs_to_send() {
                node_queue.push(node.get_id().unwrap().into());
                go_on = true;
            }
        });

        while go_on {
            for &sender in &node_queue {
                let mmsg = self.nodes[sender].send_msg();
                if mmsg.is_some() {
                    let msg = mmsg.unwrap();
                    if msg.broadcast {
                        let neighs = self.nodes[msg.from.into()].neighs();
                        for &neigh in &neighs {
                            // receive returns a message if neigh has to send an answer to the sender
                            if let Some(ans) = self.nodes[neigh.into()].receive_msg(&msg) {
                                assert_eq!(ans.from, sender.into());
                                self.nodes[sender].receive_msg(&ans);
                            }
                        }
                    }
                }
            }

            node_queue.clear();

            go_on = false;

            self.nodes.iter().for_each(|node| {
                if node.needs_to_send() {
                    node_queue.push(node.get_id().unwrap().into());
                    go_on = true;
                }
            });
        }
        0
    }
}
