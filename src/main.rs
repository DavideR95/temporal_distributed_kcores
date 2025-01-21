extern crate bitvec;
extern crate meval;

use bitvec::prelude::*;
use std::io::{BufRead, Write};

mod completely_new_algorithm;
mod dynamic_montresor;
mod temporal_graph_memory;

fn read_lines_to_vec<P>(filename: P) -> std::io::Result<Vec<String>>
where
    P: AsRef<std::path::Path>,
{
    let file = std::fs::File::open(filename).unwrap();
    let reader = std::io::BufReader::new(file);
    let mut lines = Vec::new();
    for line in reader.lines() {
        lines.push(line.unwrap());
    }
    Ok(lines)
}
fn main() {
    if std::env::args().len() < 3 {
        eprintln!("Usage: cargo run -- <path_to_graph> <snapshot_length>");
        // std::env::args().collect::<Vec<String>>()
        std::process::exit(-1);
    }

    // ***** CONFIG PARAMETERS AND UTILITIES ******

    let history_size: usize = 5;
    let mut montresor_stats = vec![];
    let mut old_montresor_stats = vec![];
    let intersection = |bitv: &BitVec| bitv.all();
    let union = |bitv: &BitVec| bitv.any();
    let half = |bitv: &BitVec| bitv.count_ones() >= (bitv.len() / 2);

    let func_name = "intersection";
    //let func_name = "union";
    //let func_name = "half";

    // ***** COMMAND LINE PARSING *****

    let args = std::env::args().collect::<Vec<String>>();
    let graph_path = &args[1];
    // let _k = args[2].parse::<usize>().unwrap();
    let snap_length = meval::eval_str(&args[2]).unwrap_or_else(|e| {
        eprintln!("Error evaluating expression: {}", e);
        std::process::exit(1);
    }) as usize;

    // ***** END: COMMAND LINE PARSING *****

    // ***** READ GRAPH FROM FILE *****

    let lines = read_lines_to_vec(graph_path).unwrap();
    let mut triplets: Vec<(usize, usize, usize)> = Vec::new();
    for line in lines {
        let parts: Vec<&str> = line.split_whitespace().collect();
        let u = parts[0].parse::<usize>().unwrap();
        let v = parts[1].parse::<usize>().unwrap();
        let t = parts[2].parse::<usize>().unwrap();
        triplets.push((u, v, t));
    }

    triplets.sort_unstable_by(|a, b| {
        a.2.cmp(&b.2)
            .then_with(|| a.0.cmp(&b.0))
            .then_with(|| a.1.cmp(&b.1))
    });

    let mut min_timestamp = triplets[0].2;
    let mut i: usize = 0;
    let mut max_nodes: usize = 0;
    let mut buckets: Vec<Vec<(usize, usize)>> = vec![vec![]; 1];
    let mut edges = 0;

    for &(u, v, timestamp) in &triplets {
        if u == v {
            continue; // No self loop
        }
        max_nodes = std::cmp::max(max_nodes, std::cmp::max(u, v));
        if timestamp >= min_timestamp + snap_length {
            min_timestamp = timestamp;
            i += 1;
            buckets.push(vec![]);
        }
        buckets[i].push((u, v));
        edges += 1;
    }

    // ***** END: READ GRAPH FROM FILE *****
    // ***** GRAPH INSTANCES ******
    let mut galive: temporal_graph_memory::AliveGraph<usize, _> =
        temporal_graph_memory::AliveGraph::from_n_and_mem_size(
            max_nodes + 1,
            history_size,
            intersection, /* bitv.count_ones() > (bitv.len() / 2)) */
        );
    let mut dyn_graph: dynamic_montresor::AliveGraph<usize, _> =
        dynamic_montresor::AliveGraph::from_n_and_mem_size(
            max_nodes + 1,
            history_size,
            intersection, /* bitv.count_ones() > (bitv.len() / 2)) */
        );
    // dyn_graph.set_m(edges);
    galive.set_m(edges);

    // ***** END: GRAPH INSTANCES *****

    // ***** ALGORITHM CORE *****

    let mut old_cores_collection = vec![];
    let mut cores_collection = vec![];
    let mut density: Vec<f32> = vec![];
    i = 0;
    let mut mism: usize = 0;
    let mut mism_coreness = 0;
    let mut mism_per_epoch = vec![];
    let mut mism_coreness_per_epoch = vec![];
    for timestamp in &buckets {
        // Add the new snapshot to the graph (batch of edges)
        print!("{i}/{}...\r", buckets.len());
        i += 1;
        std::io::stdout().flush().expect("Error flushing");
        galive.new_snapshot(timestamp);
        // Run Montresor algorithm and gather statistics
        if i >= history_size {
            // print!("\nPre-montre i={i} ");
            // galive.print_status();
            old_montresor_stats.push(galive.montresor_full());
            old_cores_collection.push(galive.k_core_from_coreness());
            // print!("Post-montre i={i}");
            // galive.print_status();
        }
        dyn_graph.new_snapshot(timestamp);
        if i >= history_size {
            // print!("dyngraph-pre: ");
            // dyn_graph.print_status();
            montresor_stats.push(dyn_graph.montresor_step());
            cores_collection.push(dyn_graph.k_core_from_coreness());
            // print!("dyngraph-post: ");
            // dyn_graph.print_status();
        }
        if i >= history_size {
            mism_per_epoch.push(0);
            mism_coreness_per_epoch.push(vec![]);
            for v in 0..galive.get_n() {
                if galive.get_node(v).get_coreness() != dyn_graph.get_node(v).get_coreness() {
                    assert!(galive.get_node(v).get_id().unwrap() == v);
                    mism += 1;

                    *mism_per_epoch.last_mut().unwrap() += 1;

                    mism_coreness +=
                        galive.get_node(v).get_coreness() - dyn_graph.get_node(v).get_coreness();

                    mism_coreness_per_epoch.last_mut().unwrap().push(
                        galive.get_node(v).get_coreness() - dyn_graph.get_node(v).get_coreness(),
                    );

                    // println!("\n\n [it={i}] Errore: il nodo {v} doveva avere coreness {}, mentre invece ha coreness {}.",galive.get_node(v).get_coreness(),dyn_graph.get_node(v).get_coreness());
                    // let mut v1 = galive.get_node(v).neighs();
                    // let mut v2 = dyn_graph.get_node(v).neighs();
                    // println!("Vicini di {v} nel vecchio: {:?}", {
                    //     v1.sort();
                    //     &v1
                    // });
                    // println!("Vicini di {v} nel nuovo: {:?}", {
                    //     v2.sort();
                    //     &v2
                    // });
                    // println!(
                    //     "Coreness di 5590: {} supposed, {} actual ",
                    //     galive.get_node(5590).get_coreness(),
                    //     dyn_graph.get_node(5590).get_coreness()
                    // );
                    // panic!("iters: {}", montresor_stats.last().unwrap().0);
                }
            }
        }
        density.push(
            (2. * timestamp.len() as f32)
                / (dyn_graph.get_n() as f32 * (dyn_graph.get_n() as f32 - 1.)),
        );
    }

    if mism > 0 {
        println!(
        "\nMismatch totali: {mism}, errore medio per epoch: +-{}, errori per iterazione (media): {}",
        mism_coreness / mism,
        mism as f64 / buckets.len() as f64
        );
    } else {
        println!("No mismatch found.");
    }

    // Jaccard similarity wrt previous bucket
    let mut similarities = vec![0.0; buckets.len()]; // Inizializza il risultato con 0.0

    for i in 1..buckets.len() {
        let set1: std::collections::HashSet<_> = buckets[i].iter().cloned().collect();
        let set2: std::collections::HashSet<_> = buckets[i - 1].iter().cloned().collect();

        let intersection_size = set1.intersection(&set2).count();
        let union_size = set1.union(&set2).count();

        similarities[i] = if union_size == 0 {
            0.0
        } else {
            intersection_size as f64 / union_size as f64
        };
    }

    // let mut mism = 0;
    // let mut x: usize = 0;
    // let mut y: usize = 0;
    // let mut z: usize = 0;

    // assert_eq!(old_cores_collection.len(), cores_collection.len());
    // for i in history_size..old_cores_collection.len() {
    //     if old_cores_collection[i].len() != cores_collection[i].len() {
    //         println!(
    //             "\n\n La i è {i} e old è = {:?}\n\n{:?}",
    //             old_cores_collection[i], cores_collection[i]
    //         );
    //         panic!("la i è {i}");
    //     }
    // }

    // for (old_core, core) in old_cores_collection.iter().zip(cores_collection.iter()) {
    //     y = 0;
    //     for (old_elem, elem) in old_core.iter().zip(core.iter()) {
    //         z = 0;
    //         for (old_sub_elem, sub_elem) in old_elem.iter().zip(elem.iter()) {
    //             //println!("a");
    //             if !elem.contains(old_sub_elem) {
    //                 println!(
    //                     "Mismatch found: old_sub_elem = {:?} (cness: {}), sub_elem = {:?} (cness: {})",
    //                     old_sub_elem, galive.get_node(*old_sub_elem).get_coreness(), sub_elem, dyn_graph.get_node(*old_sub_elem).get_coreness()
    //                 );
    //                 // // println!(
    //                 // //     "Mismatch in vectors: old_elem = {:?} (cness: {}), elem = {:?} (cness",
    //                 // //     old_elem, elem
    //                 // // );
    //                 mism += 1;
    //                 eprintln!("{old_elem:?} vs. {elem:?} --- x,y,z: {x} - {y} - {z}",);
    //                 panic!();
    //                 // eprintln!("Mismatch found in sub-elements of core elements.");
    //             }
    //             z += 1;
    //         }
    //         y += 1;
    //     }
    //     x += 1;
    // }
    // println!("All cores collections match. {mism}");

    // ***** SAVE STATS TO FILE *****

    let graph_os_string = graph_path.clone() + func_name;
    let graph_os_path = std::path::Path::new(&graph_os_string);
    let graph_name = graph_os_path.file_stem().unwrap();
    let write_header_alive = true;

    // If the results file did not exist, write the csv header too
    // if !std::path::Path::new(&format!(
    //     "results/complete_results_{}_{func_name}.csv",
    //     graph_name.to_string_lossy()
    // ))
    // .exists()
    // {
    //     write_header_alive = true;
    // }
    // write_header_alive = true;

    // STAMPARE RISULTATI DI DYNGRAPH E VEDERE SE COMBACIANO

    let mut outfile_alive = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(format!(
            "results/full_results_{}_{func_name}.csv",
            graph_name.to_string_lossy()
        ))
        .unwrap();

    let mut outfile_dynamic = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(format!(
            "results/new_results_{}_{func_name}.csv",
            graph_name.to_string_lossy()
        ))
        .unwrap();

    if write_header_alive {
        writeln!(
            outfile_alive,
            "nodes,edges,density,similarity_with_previous,bucket_length,bucket_num,mem_size,iters,total_msg,activated_nodes,msg_per_iter,time_per_iter"
        )
        .unwrap();
    }

    writeln!(
        outfile_dynamic,
        "nodes,edges,density,similarity_with_previous,bucket_length,bucket_num,mem_size,iters,total_msg,activated_nodes,msg_per_iter,time_per_iter,errors,mean_error,errors_per_iter"
    )
    .unwrap();

    i = 0;
    old_montresor_stats.iter().for_each(|result| {
        // result.0 // num iter
        // result.1 // global num of msg
        // result.2 // vector of msg per iteration
        // result.3 // time per iteration (ms)
        // result.4 // nodes that have been activated during this execution, i.e. sent > 0 msgs
        writeln!(
            outfile_alive,
            "{},{},{},{},{},{},{},{},{},{},\"{:?}\",\"{:?}\"",
            galive.get_n(),
            galive.get_m(), // Total number of edges added to the graph
            density[i],
            similarities[i],
            snap_length,
            buckets.len(),
            history_size,
            result.0,
            result.1,
            result.4,
            result.2,
            result
                .3
                .iter()
                .map(|d| d.as_millis())
                .collect::<Vec<u128>>(),
        )
        .unwrap();
        i += 1;
    });

    i = 0;
    montresor_stats.iter().for_each(|result| {
        // result.0 // num iter
        // result.1 // global num of msg
        // result.2 // vector of msg per iteration
        // result.3 // time per iteration (ms)
        // result.4 // nodes that have been activated during this execution, i.e. sent > 0 msgs
        writeln!(
            outfile_dynamic,
            "{},{},{},{},{},{},{},{},{},{},\"{:?}\",\"{:?}\",{},{},{}",
            dyn_graph.get_n(),
            dyn_graph.get_m(), // Total number of edges added to the graph
            density[i],
            similarities[i],
            snap_length,
            buckets.len(),
            history_size,
            result.0,
            result.1,
            result.4,
            result.2,
            result
                .3
                .iter()
                .map(|d| d.as_millis())
                .collect::<Vec<u128>>(),
            mism_per_epoch[i],
            if mism_coreness_per_epoch[i].is_empty() {
                0.0
            } else {
                mism_coreness_per_epoch[i].iter().sum::<usize>() as f64
                    / mism_coreness_per_epoch[i].len() as f64
            }, //if mism > 0 { mism_coreness / mism } else { 0 },
            mism as f64 / buckets.len() as f64,
        )
        .unwrap();
        i += 1;
    });

    outfile_dynamic.flush().unwrap();
    outfile_alive.flush().unwrap();

    println!("All done.");
}
