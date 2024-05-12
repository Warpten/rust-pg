use nohash_hasher::IntMap;

use crate::utils::topological_sort;

use super::{manager::Identifiable, pass::{Pass, PassID}, resource::ResourceID, Graph};

pub struct DefaultScheduler;

pub trait Scheduler {
    fn schedule(graph : &Graph, topological_sort : Vec<&Pass>);
}

#[derive(Clone)]
struct QueueScheduler {
    pub passes : Vec<PassID>,
    pub full : bool,
}

impl QueueScheduler {
    pub fn new(pass_count : usize) -> Self {
        Self {
            passes : vec![PassID::NONE; pass_count],
            full   : false,
        }
    }

    pub fn can_process(&self, pass : &Pass) -> bool {
        todo!()
    }

    pub fn take(&self, pass : &Pass) {
        todo!()
    }
}

impl Scheduler for DefaultScheduler {
    fn schedule(graph : &Graph, topological_sort : Vec<&Pass>) {
        // All queued passes, indexed on the exact queue index (ignoring families)
        let queue_schedulers = vec![QueueScheduler::new(topological_sort.len()); 2];
        
        // Create a set of synchronization rules
        for pass in &topological_sort {
            match queue_schedulers.iter().find(|scheduler| scheduler.can_process(pass)) {
                Some(scheduler) => scheduler.take(pass),
                None => panic!("No queue can accept this pass")
            }
        }

        while !topological_sort.is_empty() {
            let pass = &topological_sort[0];

            // Use the first pass using the current one as input as a marker to find strings of passes
            // that are not related.
            let unconnected_blocks = topological_sort.iter().skip(1).take_while(|other| {
                // Look for that pass's inputs
                !other.inputs().iter().find(|resource| {
                    // If one if these inputs is the current pass's output, there is a dependency.
                    match resource {
                        ResourceID::Virtual(output, _) if *output == pass.id() => true,
                        _ => false
                    }
                }).is_some()
            }).collect::<Vec<_>>();

            // Every pass in between
        }

        for i in 0..topological_sort.len() {
            let pass = &topological_sort[i];

            // For every pass after this one in the topology...
            for j in (i + 1)..topological_sort.len() {
                let other = &topological_sort[j];

                // If said other pass reads from us...
                let links = other.inputs().iter().find(|resource| {
                    match resource {
                        ResourceID::Virtual(output, _) if *output == pass.id() => true,
                        _ => false
                    }
                });
            }
        }

        // Start by pushing all the nodes to the same queue.

        // (pass, (prev, next))
        let mut synchronizations = IntMap::<PassID, (Vec<PassID>, Vec<PassID>)>::default();

        for pass in topological_sort {
            for input in pass.inputs() {
                if let ResourceID::Virtual(output, _) = input {
                    synchronizations.entry(pass.id()).or_insert((vec![], vec![])).0.push(output);
                    synchronizations.entry(output).or_insert((vec![], vec![])).1.push(pass.id());
                }
            }
        }
    }
}