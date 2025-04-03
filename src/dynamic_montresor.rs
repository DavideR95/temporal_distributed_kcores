use bitvec::prelude::*;
use std::collections::HashMap;
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
    estimate: HashMap<T, (usize, usize)>, // First value: estimate, second: timestamp
    activation_function: F,
    time: usize,
    activated: bool,
    dirty: bool,
    paused: bool,
    last_sent: usize,
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
            time: 0,
            activated: false,
            paused: false,
            dirty: false,
            last_sent: 0,
            old_coreness: 0,
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

    pub fn apply_f(&self, who: &BitVec) -> bool {
        (self.activation_function)(&who[who.len().saturating_sub(self.memory_size)..].to_bitvec())
    }
    pub fn has_changed(&self) -> bool {
        if self.id.is_some() {
            return self.changed;
        }
        false
    }

    pub fn advance_time(&mut self) {
        self.time += 1;
        for neigh in self.neighbors.values_mut() {
            neigh.push(false);
        }
        self.old_coreness = self.coreness;
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

    pub fn reset(&mut self) {
        if self.id.is_some() {
            let my_neighs = self.neighs();
            let keys: Vec<T> = self.estimate.keys().cloned().collect();
            let mut lost_neigh = false;

            // if self.id.unwrap() == 1103.into() {
            //     println!("sono l' {}", self.id.unwrap());
            // }

            for neigh in &keys {
                if !my_neighs.contains(neigh) {
                    self.estimate.remove(neigh);
                    lost_neigh = true;
                    // if self.id.unwrap() == 1103.into() {
                    //     println!("\tho perso un neigh, avevo coreness {}", self.coreness);
                    // }
                }
            }

            if self.dirty {
                self.coreness = self.deg();
                self.changed = true;
                self.dirty = false;
                // if self.id.unwrap() == 6841.into() || self.id.unwrap() == 1103.into() {
                //     println!("\tripristino la coreness al deg cioè {}", self.coreness);
                // }
            } else if lost_neigh {
                let old_cness = self.coreness;
                self.coreness = self.comp_coreness();
                if self.coreness == 0 {
                    self.coreness = usize::MAX;
                } else if self.coreness != old_cness {
                    self.changed = true;
                    // if self.id.unwrap() == 6841.into() {
                    //     println!(
                    //     "\tnuova coreness diversa da old coreness cioè {} diverso da {old_cness}",
                    //     self.coreness
                    // );
                    // }
                }
            } /*  else if self.time > self.last_sent + 1 {
                  self.coreness = self.deg();
                  self.estimate.values_mut().for_each(|e| e.0 = usize::MAX);
                  self.changed = true;
              }*/

            // self.changed = true;
            // self.coreness = self.deg();
            // if self.coreness == 0 {
            //     self.coreness = usize::MAX;
            // }
            // if self.id.unwrap() == 1.into() {
            //     println!("Sono passato di qui?");
            // }

            self.count.resize(self.estimate.len() + 1, 0);
        }
    }

    pub fn get_time(&self) -> usize {
        self.time
    }

    pub fn receive_msg(&mut self, from: T, core: usize, timestamp: usize) {
        let mut less_than = false;
        let mut is_reset = false;
        // if self.id.unwrap() == 1675.into() {
        //     print!("sono il 1675, ricevo da {from} il valore {core} con time {timestamp}. ");
        //     println!("I miei neigh sono: ");
        //     for neigh in &self.neighbors {
        //         print!("{} - {:} {}, ", neigh.0, neigh.1, self.apply_f(neigh.1));
        //     }

        //     print!("la sua estimate è: {:?}", self.estimate);
        //     println!("");
        // }

        // if self.id.unwrap() == 6841.into()
        //     || from == 1103.into()
        //     || self.id.unwrap() == 1103.into()
        //     || from == 6841.into()
        // {
        //     println!("Sono il {} e ricevo: {core} da {from}", self.id.unwrap());
        // }

        match self.estimate.get(&from) {
            Some(&est) => {
                if (core < est.0 && timestamp == est.1) || (core > est.0 && timestamp > est.1) {
                    less_than = true;
                } else if timestamp > est.1 {
                    is_reset = true;
                }
            }
            None => less_than = true,
        }

        if less_than || is_reset {
            self.estimate.insert(from, (core, timestamp));
            // println!("adesso la estimate è: {:?} ", self.estimate);
            let t = self.comp_coreness(); // Compute index
                                          // println!(
                                          //     "nodo: {}, t = {t}, comp_coreness = {}, my degree = {}",
                                          //     self.id.unwrap(),
                                          //     self.comp_coreness(),
                                          //     self.deg()
                                          // );
                                          // if self.id.unwrap() == 6841.into()
                                          //     || self.id.unwrap() == 1103.into()
                                          //     || from == 1103.into()
                                          // {
                                          //     println!(
                                          //         "[{}] comp coreness: {}, my estimate: {:?}",
                                          //         self.id.unwrap(),
                                          //         t,
                                          //         self.estimate
                                          //     );
                                          // }
            if t != self.coreness {
                // if self.id.unwrap() == 6841.into() || self.id.unwrap() == 1103.into() {
                //     println!(
                //     "Il nodo {} cambia la sue coreness in {} (prima era {}) dopo aver ricevuto ({from},{core})",
                //     self.id.unwrap(),
                //     t,self.coreness);
                // }
                if t < self.coreness {
                    self.paused = true;
                    // if self.id.unwrap() == 6841.into() || self.id.unwrap() == 1103.into() {
                    //     println!("Il nodo {} si mette in pausa", self.id.unwrap());
                    // }
                }
                self.coreness = t;

                self.changed = true;
                self.activated = true;
            }
        }
    }

    pub fn comp_coreness(&mut self) -> usize {
        let mut count = vec![0; self.estimate.len() + 1];

        // if self.id.unwrap() == 1.into() {
        //     print!("sono il 1 la mia coreness attuale è {} ", self.coreness);
        // }

        for &core in self.estimate.values() {
            if core.0 < usize::MAX {
                if core.0 < count.len() {
                    count[core.0] += 1;
                } else {
                    count[self.estimate.len()] += 1;
                }
            } else {
                return std::cmp::min(self.coreness, self.deg());
                //count[self.coreness] += 1;
            }
        }

        let mut total = 0;
        for i in (0..count.len()).rev() {
            total += count[i];
            if total >= i {
                // if self.id.unwrap() == 1.into() {
                //     println!("e quella nuova è {}", i);
                // }
                if i == 0 {
                    return usize::MAX;
                }
                return i;
            }
        }
        //println!("\tMa io restituisco {}", self.deg());
        self.deg()
    }

    // Alg 1 from Montresor paper
    // Last usize is the time of the msg
    pub fn send_msg(&mut self) -> Option<(T, usize, usize)> {
        match self.id {
            None => return None,
            Some(_) => {
                if self.changed && !self.paused {
                    //if self.changed {
                    self.changed = false;
                    self.activated = true;
                    // if self.id.unwrap() == 6841.into() || self.id.unwrap() == 5417.into() {
                    //     println!(
                    //         "Sono il {} e invio adesso un messaggio con: (c={},t={})",
                    //         self.id.unwrap(),
                    //         self.coreness,
                    //         self.time
                    //     );
                    // }
                    self.last_sent = self.time;
                    return Some((self.id.unwrap(), self.coreness, self.time));
                //}
                } else if self.paused {
                    self.paused = false;
                    // if self.id.unwrap() == 1103.into() {
                    //     println!("Sono pausato, non invio");
                    // }
                }
                return None;
            }
        }
    }

    pub fn get_estimate(&self) -> Vec<T> {
        self.estimate.keys().cloned().collect()
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
            time: self.time,
            activated: self.activated,
            dirty: self.dirty,
            paused: self.paused,
            last_sent: self.last_sent,
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

    pub fn get_node(&self, id: usize) -> &AliveNode<T, F> {
        &self.nodes[id]
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

    pub fn print_status(&mut self) {
        for node in &mut self.nodes {
            if node.get_id().is_some()
                && (node.get_id().unwrap() == 1103.into() || node.get_id().unwrap() == 5590.into())
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
                println!("] coreness computata: {}", node.comp_coreness());
            }
        }
    }

    pub fn get_neighs(&self, who: usize) -> Vec<usize> {
        let mut ans: Vec<usize> = self.nodes[who]
            .neighs()
            .into_iter()
            .map(|x| x.into())
            .collect();
        ans.sort_unstable();

        ans
    }

    pub fn changed_coreness_count(&self) -> (usize, usize) {
        let mut count = 0;
        let mut isolated = 0;
        self.nodes.iter().for_each(|n| {
            if n.id.is_some() && n.coreness != n.old_coreness {
                count += 1;
            }
            if n.id.is_some() && n.coreness == usize::MAX && n.old_coreness < usize::MAX {
                isolated += 1;
            }
        });

        (count, isolated)
    }

    pub fn montresor_step(
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
            if node.has_changed() {
                if node.get_coreness() >= usize::MAX {
                    let deg = node.deg();
                    if deg > 0 {
                        node.coreness = deg;
                    } else {
                        node.changed = false;
                        continue; // Skip this node if it is isolated
                    }
                } else if node.deg() <= 0 {
                    node.coreness = usize::MAX;
                    node.changed = false;
                    continue;
                }
                node_queue.push(node.get_id().unwrap().into());
            }
        }
        // END: Initialization

        // START: Main loop
        let mut msg_queue: Vec<(T, usize, usize)> = vec![];
        msg_queue.reserve(self.get_n());

        while go_on {
            // New iteration
            // println!("Next iter");
            num_iterations += 1;
            msgs_per_iteration.push(0);

            // Chrono
            let start = std::time::Instant::now();

            // Gather messages (node, coreness) from every node
            // that needs to send an update
            for &node in &node_queue {
                if let Some((id, cness, timestamp)) = self.nodes[node].send_msg() {
                    msg_queue.push((id, cness, timestamp));
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
                    self.nodes[neigh].receive_msg(msg.0, msg.1, msg.2);
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
