use std::collections::VecDeque;

use nohash_hasher::IntMap;

use crate::{utils::topological_sort, Queue};

use super::{manager::Identifiable, pass::{Pass, PassID}, resource::ResourceID, Graph};

pub struct DefaultScheduler;

pub trait Scheduler {
    fn schedule(graph : &Graph, topological_sort : Vec<&Pass>, queues : &Vec<Queue>);
}

/// Synthesized passes. Can be either a reference to an actual graph pass, or a synchronization node
enum SyntheticPass {
    Pass(PassID),
    CrossQueueSynchronization(PassID, PassID),
}

struct QueueScheduler<'a> {
    pub passes : Vec<PassID>,
    pub queue : &'a Queue,
    pub enqueuing_index : usize,
}

impl QueueScheduler<'_> {
    pub fn new<'a>(queue : &'a Queue, pass_count : usize) -> QueueScheduler<'a> {
        QueueScheduler {
            queue,
            passes : vec![PassID::NONE; pass_count],
            enqueuing_index : 0,
        }
    }

    pub fn can_process(&self, pass : &Pass) -> bool {
        if self.enqueuing_index < self.passes.len() {
            return true;
        }

        false
    }

    pub fn append(&mut self, pass : &Pass) {
        self.passes[self.enqueuing_index] = pass.id();
        self.enqueuing_index += 1;
    }
}

impl Scheduler for DefaultScheduler {
    fn schedule(graph : &Graph, topological_sort : Vec<&Pass>, queues : &Vec<Queue>) {
        // All queued passes, indexed on the exact queue index (ignoring families)
        let queue_schedulers = queues.iter().map(|q| QueueScheduler::new(&q, topological_sort.len())).collect::<Vec<_>>();
        
        // A queue of the passes that can be reordered at any point in the graph.
        let reorganizable_passes = VecDeque::<PassID>::with_capacity(topological_sort.len());
        
        // My basic idea (god knows if it's actually good, though) is to find the first and the last pass in the topology
        // that are directly connected to the starting pass.
        // We then a) spread out these nodes across all available queues (taking note of a queue accepting to process a given
        // pass). If we have less queues available than passes to spread, we can sequence them with each other and this won't
        // break sequencing because we now have implicit sequencing.
        // For example, these are equivalent in terms of synchronization between passes
        //         +---+                 +---+   +---+                  +---+
        //         | 2 |                 | 2 | → | 4 |                  | 2 |
        // +---+ ↗ +---+         +---+ ↗ +---+   +---+          +---+ ↗ +---+
        // | 1 | → | 3 |         | 1 |                          | 1 |
        // +---+ ↘ +---+         +---+ ↘ +---+                  +---+ ↘ +---+   +---+
        //         | 4 |                 | 3 |                           | 3 | → | 4 |
        //         +---+                 +---+                           +---+   +---+
        //       (A)                         (B)                             (C)
        // With that in mind, we can now add * *the minimum** amount of any of the passes that were not sequenced after 
        // pass 3 on (B) (or pass 2 on (C)) (again, taking in mind wether or not the queues can accept these passes). These passes
        // could also be sequenced with one pass before 3 on (B) (or 2 on (C)) because 1 is happening on a single queue (regardless
        // of how the figures above lays it out), or any pass before 2 and 3 and with 1. This is to prevent whatever queues not
        // executing 1 from stalling until 1 completes because whatever pass is waiting for 1 on the other queues.
        // Once that's done, we keep track of the passes that could not be injected, and repeat this process. This time, we're looking
        // for passes that are connected to the the tail of each queue, and sequencing on top of it. We keep going until all passes
        // have been assigned a queue. From there, all we have to do is inject synchronization points across queues wherever needed;
        // but there is one final thing we need to worry about: redundant synchronization. This can arise especially on B (or C, but
        // let's only consider B for this example) if whatever links to passes 2 and 4 is only linking to one of both.
        // Given the following graph:
        //    0
        //  / | \
        // 1  2  3
        // | /|  |
        // 4  5  6
        // |   \ |
        // 8     7
        // When spread on two queues this can become:
        // 0  
        // | \
        // 1  2 
        // |  |
        // 3  5
        // | /|
        // 4  6
        // |  |
        // 8  7
        // Note that now we have an explicit synchronization between 5 and 6. This means that synchronizations have now become late binding!
        // 5 was sequenced to 3, but 5 waiting on 3 would stall the execution of 6, so 5 waits for 6 now, which is itself implicitely waiting
        // on 3, since it's on the same queue.

        {
            let mut orphan_queue = VecDeque::<PassID>::with_capacity(topological_sort.len());

            let mut index = 0;
            let mut queue_roundtrip_index = 0;
            while index < topological_sort.len() {
                let current = topological_sort[index];
                index += 1; 

                { // If the pass does not take any virtual input it can be reordered anywhere on the graph.
                    let is_orphaned = current.inputs().iter().all(|input| {
                        match input {
                            ResourceID::Texture(_) => true,
                            ResourceID::Buffer(_) => true,
                            ResourceID::None => true,
                            ResourceID::Virtual(_, _) => false,
                        }
                    });

                    if is_orphaned {
                        orphan_queue.push_back(current.id());
                    } else {
                        
                    }
                }

                // Find all passes taking at least one input from any of this pass's outputs.
                let edges = topological_sort.iter().filter(|pass| {
                    pass.inputs().iter().any(|input| {
                        match input {
                            ResourceID::Virtual(src, _) if *src == current.id() => true,
                            _ => false,
                        }
                    })
                }).collect::<Vec<_>>();
            }
        }

        
        // For the purpose of this algorithm, because freestanding (as in,
        // independant) nodes are pushed to the top of the topology, we can consider them to be connected to an imaginary root
        // node that sits further ahead of them in the topology.

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