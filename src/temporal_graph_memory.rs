#![allow(dead_code)]

use bitvec::prelude::*;
use std::collections::{HashMap, HashSet};
use std::fmt::Debug;
use std::hash::Hash;

pub struct AliveNode<T, F>
where
    T: Clone + Copy + Into<usize> + std::fmt::Display + Ord + std::convert::From<usize> + Hash,
    F: Fn(&BitVec) -> bool + std::clone::Clone,
{
    // Node-specific attributes
    id: Option<T>,                 // Node id, if None the node is not present
    neighbors: HashMap<T, BitVec>, // Keeps track of the last t snapshots of neighbors
    memory_size: usize,
    // Montresor Variables
    changed: bool,
    coreness: usize,
    count: Vec<usize>,
    estimate: HashMap<T, usize>, // First value: estimate
    activation_function: F,
    activated: bool,
    old_coreness: usize,
}

impl<T, F> AliveNode<T, F>
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
        AliveNode {
            id: nid,
            neighbors: HashMap::new(),
            changed: false,
            coreness: usize::MAX,
            estimate: HashMap::new(),
            count: vec![],
            memory_size: _memory_size,
            activation_function: af,
            activated: false,
            old_coreness: usize::MAX,
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
            .filter(|key| (self.activation_function)(&self.neighbors[key]))
            .collect()
    }

    pub fn apply_f(&self, who: &BitVec) -> bool {
        (self.activation_function)(who)
    }

    pub fn has_changed(&self) -> bool {
        if self.id.is_some() {
            return self.changed;
        }
        false
    }

    pub fn advance_time(&mut self) {
        let mut to_remove = vec![];

        self.old_coreness = self.coreness;

        // self.estimate.clear();

        for (neighbor, bitvec) in self.neighbors.iter_mut() {
            assert!(bitvec.len() == self.memory_size);
            bitvec.shift_left(1);
            if bitvec.not_any() {
                to_remove.push(*neighbor);
            }
        }

        for neighbor in to_remove {
            self.neighbors.remove(&neighbor);
            self.estimate.remove(&neighbor);
        }
    }

    pub fn clear_estimate(&mut self) {
        self.estimate.values_mut().for_each(|e| *e = usize::MAX);
    }

    pub fn add_neighbor_latest_time(&mut self, neighbor: T) {
        self.neighbors
            .entry(neighbor)
            .and_modify(|bits| {
                bits.last_mut().unwrap().set(true);
                if !(self.activation_function)(bits) {
                    self.estimate.remove(&neighbor);
                    // Se il neighbor non è più raggiungibile, lo tolgo
                } else {
                    // Se resta raggiungibile, non fare niente
                    // Se lo avevo tolto, lo reinserisco con infinito
                    self.estimate.entry(neighbor).or_insert(usize::MAX);
                }
            })
            .or_insert_with(|| {
                let mut bitvec = BitVec::with_capacity(self.memory_size);
                bitvec.resize(self.memory_size, false);
                bitvec.last_mut().unwrap().set(true);
                if (self.activation_function)(&bitvec) {
                    self.estimate.insert(neighbor, usize::MAX);
                }
                bitvec
            });

        self.count.resize(self.estimate.len() + 1, 0);
    }

    pub fn receive_msg(&mut self, from: T, core: usize) {
        let mut less_than = false;

        // if self.id.unwrap() == 1.into() {
        //     println!("[old] Sono 1 e ricevo {core} da {from}");
        // }

        match self.estimate.get(&from) {
            Some(&est) => {
                if core <= est {
                    less_than = true;
                }
            }
            None => less_than = true,
        }

        // Usando graph_temp.txt qui al primo passaggio la coreness di 0 torna a 3
        // E non salirà più a 4
        if less_than {
            self.estimate.insert(from, core);
            // if self.id.unwrap() == 1.into() {
            //     println!("\t\t[old] Sono 1 e ricevo {core} da {from}");
            // }
            let t = self.compute_index(); // Compute index
                                          // if self.id.unwrap() == 1.into() {
                                          //     println!("La t è {t} ma la coreness vecchia era {}", self.coreness);
                                          // }

            if t < self.coreness {
                self.coreness = t;
                self.changed = true;
            }
        }
    }

    pub fn compute_index(&mut self) -> usize {
        //let mut count = vec![0; self.coreness + 1];
        self.count.fill(0);

        // Non aggiorno effettivamente la mia stima finché non ho ricevuto aggiornamento da tutti gli altri
        // if self.neighs().iter().any(|e| self.estimate[e].1 < self.time) {
        //     return self.coreness;
        // }

        // if self.estimate.iter().any(|e| (e.1).1 < self.time) {
        //     return self.coreness;
        // }
        for neigh in self.neighs() {
            //if self.estimate[&neigh].1 >= self.time {
            //let mut j = self.coreness;
            //if self.estimate[&neigh].1 >= self.time {
            let j = std::cmp::min(self.coreness, self.estimate[&neigh]);
            //}
            self.count[j] += 1;
            //}
        }
        // for neigh in &self.neighbors {
        //     if activation_function(neigh.1, ts, te) {
        //         let j = std::cmp::min(
        //             self.coreness,
        //             self.estimate[neigh.0], //*(self.estimate.get(neigh).unwrap_or(&usize::MAX)),
        //         );
        //         self.count[j] += 1;
        //     }
        // }

        let mut i = self.coreness;

        while i > 0 {
            self.count[i - 1] = self.count[i - 1] + self.count[i];
            i -= 1;
        }

        i = self.coreness;

        while i > 0 && self.count[i] < i {
            i -= 1;
        }

        i
    }

    // Alg 1 from Montresor paper
    // Last usize is the time of the msg
    pub fn send_msg(&mut self) -> Option<(T, usize)> {
        match self.id {
            None => return None,
            Some(_) => {
                if self.changed {
                    self.changed = false;
                    self.activated = true;
                    return Some((self.id.unwrap(), self.coreness));
                }
                return None;
            }
        }
    }
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
            activated: self.activated,
            old_coreness: self.old_coreness,
        }
    }
}

impl<T, F> std::fmt::Debug for AliveNode<T, F>
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
#[derive(Debug)]
pub struct AliveGraph<T, F>
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
    nodes: Vec<AliveNode<T, F>>,
    edges: usize,
    memory_size: usize,
    activation_function: F,
}

impl<T, F> AliveGraph<T, F>
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
        AliveGraph {
            nodes: vec![],
            edges: 0,
            memory_size: 5,
            activation_function,
        }
    }

    pub fn from_n_and_mem_size(nid: usize, mem_size: usize, _activation_function: F) -> Self {
        AliveGraph {
            nodes: vec![AliveNode::new(None, mem_size, _activation_function.clone()); nid],
            edges: 0,
            memory_size: mem_size,
            activation_function: _activation_function,
        }
    }

    pub fn get_n(&self) -> usize {
        let mut count = 0;
        self.nodes.iter().for_each(|n| {
            if n.get_id().is_some() {
                count += 1
            }
        });
        count
    }

    pub fn get_node(&self, id: usize) -> &AliveNode<T, F> {
        &self.nodes[id]
    }

    pub fn new_snapshot(&mut self, list: &Vec<(T, T)>) {
        self.nodes.iter_mut().for_each(|node| node.advance_time());

        let mut to_reset: HashSet<usize> = HashSet::new();

        for &(u, v) in list {
            if u == v {
                continue;
            }

            match self.nodes[u.into()].get_id() {
                Some(_) => (),
                None => self.nodes[u.into()].set_id(u),
            }
            match self.nodes[v.into()].get_id() {
                Some(_) => (),
                None => self.nodes[v.into()].set_id(v),
            }

            self.nodes[u.into()].add_neighbor_latest_time(v);
            self.nodes[v.into()].add_neighbor_latest_time(u);

            to_reset.insert(u.into());
            to_reset.insert(v.into());

            // self.edges += 2;
        }

        // for u in to_reset {
        //     self.nodes[u].reset();
        //     // self.nodes[v.into()].reset();

        //     // for neigh_of_u in self.nodes[u].neighs() {
        //     //     self.nodes[neigh_of_u.into()].reset();
        //     // }
        //     // for neigh_of_v in self.nodes[v.into()].neighs() {
        //     //     self.nodes[neigh_of_v.into()].reset();
        //     // }
        // }
    }

    pub fn get_m(&self) -> usize {
        self.edges
    }

    pub fn set_m(&mut self, _m: usize) {
        self.edges = _m;
    }

    pub fn print_status(&mut self) {
        for node in &mut self.nodes {
            if node.get_id().is_some()
                && (node.get_id().unwrap() == 3291.into() || node.get_id().unwrap() == 1103.into())
            {
                print!(
                    "Nodo: {} corenes: {} [",
                    node.get_id().unwrap(),
                    node.coreness
                );
                for neigh in &node.neighbors {
                    print!("{} - {:} {}, ", neigh.0, neigh.1, node.apply_f(neigh.1));
                }
                print!(
                    "i veri neighs ({}): {:?}, la sua estimate è: {:?}",
                    node.neighs().len(),
                    node.neighs(),
                    node.estimate
                );
                println!("]");
            }
        }
    }

    pub fn changed_coreness_count(&self) -> (usize, usize) {
        let mut count = 0;
        let mut other_count = 0;
        self.nodes.iter().for_each(|n| {
            if n.id.is_some() && n.coreness != n.old_coreness {
                count += 1;
                //println!("{} vs {}", n.coreness, n.old_coreness);
            }
            if n.id.is_some() && n.coreness == usize::MAX && n.old_coreness < usize::MAX {
                other_count += 1;
            }
        });
        //println!("L'other è {other_count}");
        (count, other_count)
    }

    // pub fn print_status(&self) {
    //     for node in &self.nodes {
    //         if node.get_id().is_some() {
    //             let mut neighs = node.neighs();
    //             if neighs.is_empty() {
    //                 continue;
    //             }
    //             print!("{}: [", node.get_id().unwrap());
    //             neighs.sort_unstable();
    //             for neigh in &neighs {
    //                 if neighs.len() > 1 {
    //                     print!(", {}", neigh);
    //                 } else {
    //                     print!("{}", neigh);
    //                 }
    //             }
    //             println!("]");
    //         }
    //     }
    //     println!("!!");
    // }

    pub fn get_neighs(&self, who: usize) -> Vec<usize> {
        let mut ans: Vec<usize> = self.nodes[who]
            .neighs()
            .into_iter()
            .map(|x| x.into())
            .collect();
        ans.sort_unstable();

        ans
    }

    pub fn montresor_full(
        &mut self,
    ) -> (usize, usize, Vec<usize>, Vec<std::time::Duration>, usize) {
        // Housekeeping

        let expected_iterations = (|| {
            // Heuristic: get the first two digits of n
            // to be used as an estimate for the number of iterations of this algorithm
            let mut n = self.get_n();
            while n >= 100 {
                n /= 10;
            }
            n * 2
        })();

        // STATISTICS
        let mut num_iterations: usize = 0;
        let mut num_of_msgs = 0;
        let mut msgs_per_iteration: Vec<usize> = Vec::with_capacity(expected_iterations);
        let mut time_per_iteration: Vec<std::time::Duration> =
            Vec::with_capacity(expected_iterations);

        // END: Statistics
        // END: Housekeeping

        // START: Initialization

        let mut go_on = true; // True if at least one node has to send a message
        let mut node_queue: Vec<usize> = Vec::with_capacity(self.get_n());

        for node in &mut self.nodes {
            if node.get_id().is_some() {
                // assert!(node.coreness >= usize::MAX);
                node.coreness = usize::MAX;
                node.activated = false;
                // node.set_interval(ts, te, &activation_function);
                let degree = node.deg(); //_interval();
                                         // Consider a node only if it is not isolated
                if degree > 0 {
                    node.changed = true;
                    node.coreness = degree;
                    //node.activated = true;
                    node.clear_estimate();
                    node_queue.push(node.get_id().unwrap().into());
                }
            }
        }

        // self.nodes.iter().for_each(|node| {
        //     if node.has_changed() {
        //         node_queue.push(node.get_id().unwrap().into());
        //         go_on = true;
        //         //node.stabilize();
        //         // node.changed will be reset by the send_msg function (or the next initialization)
        //     }
        // });

        // END: Initialization

        // START: Main loop
        let mut msg_queue: Vec<(T, usize)> = vec![];
        msg_queue.reserve(self.get_n());

        while go_on {
            // New iteration
            num_iterations += 1;
            msgs_per_iteration.push(0);

            // Chrono
            let start = std::time::Instant::now();

            // Gather messages (node, coreness) from every node
            // that needs to send an update
            for &node in &node_queue {
                if let Some((id, cness)) = self.nodes[node].send_msg() {
                    msg_queue.push((id, cness));
                    num_of_msgs += 1;
                    msgs_per_iteration[num_iterations - 1] += 1;
                }
            }

            node_queue.clear();

            // Process messages in neighbors of the sender (msg.0)

            for msg in &msg_queue {
                // Cercare di rimuovere clone
                let neighbors = self.get_neighs(msg.0.into()); //.clone();
                for &neigh in &neighbors {
                    // No self updates
                    assert!(msg.0.into() != neigh.into());
                    // Each neigh needs to process the estimate for its neighbors
                    // Not the neighs in neighbors as they are of the sender
                    // not those of the receiver
                    self.nodes[neigh].receive_msg(msg.0, msg.1);
                }
            }

            go_on = false;
            self.nodes.iter().for_each(|node| {
                if node.has_changed() {
                    node_queue.push(node.get_id().unwrap().into());
                    go_on = true;
                    //node.stabilize();
                    // node.changed will be reset by the send_msg function (or the next initialization)
                }
            });

            let elapsed = start.elapsed();
            time_per_iteration.push(elapsed);

            msg_queue.clear();
        }

        let mut activated_nodes = 0;
        self.nodes.iter_mut().for_each(|v| {
            if v.activated {
                activated_nodes += 1;
                v.activated = false;
            }
        });

        (
            num_iterations,
            num_of_msgs,
            msgs_per_iteration,
            time_per_iteration,
            activated_nodes, // Raccogliere statistiche su questo
        )
    }

    pub fn k_core_from_coreness(&self) -> Vec<Vec<T>> {
        let mut ans: Vec<Vec<T>> = vec![vec![]; self.get_n()];

        for node in &self.nodes {
            if let Some(v) = node.get_id() {
                if node.coreness < usize::MAX {
                    ans[node.coreness].push(v);
                }
            }
        }

        while let Some(last) = ans.last() {
            if last.is_empty() {
                ans.pop();
            } else {
                break;
            }
        }

        ans
    }
}
