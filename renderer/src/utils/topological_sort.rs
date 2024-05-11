use std::{collections::{hash_map::Entry, VecDeque}, hash::Hash};

use nohash_hasher::{IntMap, IntSet};

/// Errors codes that can be returned by a topological sort of a Directed Acyclic Graph.
#[derive(Debug, Eq, PartialEq)]
pub enum Error {
    Cyclic,
}

pub struct TopologicalSorter<T : Ord + Eq + Copy + Hash + nohash_hasher::IsEnabled> {
    _nodes : Vec<T>,
    _edges : IntMap<T, Vec<T>>,
}

impl<T : Ord + Eq + Copy + Hash + nohash_hasher::IsEnabled> Default for TopologicalSorter<T> {
    fn default() -> Self {
        Self { _nodes: Default::default(), _edges: Default::default() }
    }
}

impl<T : Ord + Eq + Copy + Hash + nohash_hasher::IsEnabled> TopologicalSorter<T> {
    pub fn add_node(mut self, node : T, edges : Vec<T>) -> Self {
        self._nodes.push(node);
        self._edges.insert(node, edges);
        self
    }

    #[allow(dead_code)]
    pub fn sort_kahn(&self) -> Result<Vec<T>, Error> {
        // Build an adjacency list
        let adj = {
            let mut adj = IntMap::<T, Vec<T>>::default();
            for node in &self._nodes {
                adj.entry(*node).or_insert(vec![]); // Make sure an adjacency list exists for this node
                
                // For each edge
                for edge in &self._edges[node] {
                    // Find the corresponding adjacency list
                    match adj.entry(*edge) {
                        // Inform this edge that we are and adjacent (additively)
                        Entry::Occupied(mut value) => value.get_mut().push(*node),
                        Entry::Vacant(value) => { value.insert(vec![*node]); },
                    };
                }
            }
            adj
        };

        kahn_impl::<T>(adj)
    }

    #[allow(dead_code)]
    pub fn sort_dfs(&self) -> Result<Vec<T>, Error> {
        let mut visited = IntSet::<T>::default();
        let mut on_stack = IntSet::<T>::default();
    
        let mut sorted = Vec::<T>::default();

        let mut edge_map = IntMap::<T, &Vec<T>>::default();
        for node in &self._nodes {
            edge_map.insert(*node, &self._edges[node]);
        }

        for (node, _) in &edge_map {
            if !visited.contains(&node) {
                if dfs_impl(node, &edge_map, &mut visited, &mut on_stack, &mut sorted) {
                    return Err(Error::Cyclic);
                }
            }
        }
    
        sorted.reverse();
        Ok(sorted)
    }
}

fn kahn_impl<T : Ord + Eq + Copy + Hash + nohash_hasher::IsEnabled>(mut adjacency : IntMap<T, Vec<T>>) -> Result<Vec<T>, Error> {
    // Push vertices with no incoming edge to a queue
    let mut no_incoming_edges_queue = adjacency.iter().filter_map(|(k, v)| {
        if v.is_empty() {
            Some(k.clone())
        } else {
            None
        }
    }).collect::<VecDeque<_>>();

    let mut sorted = Vec::<T>::default(); // Output
    // While the queue is not empty, pop the queue from the back and add to output.
    while let Some(no_incoming_edges) = no_incoming_edges_queue.pop_back() {
        adjacency.remove(&no_incoming_edges);
        sorted.push(no_incoming_edges);

        // For each neighbor of 
        for (other, other_adj) in &mut adjacency {
            if !no_incoming_edges_queue.contains(other) && *other != no_incoming_edges {
                other_adj.retain(|i| *i != no_incoming_edges);
                if other_adj.is_empty() {
                    no_incoming_edges_queue.push_back(*other);
                }
            }
        }
    }

    if adjacency.is_empty() {
        Ok(sorted)
    } else {
        Err(Error::Cyclic)
    }
}

fn dfs_impl<T : Ord + Eq + Copy + Hash + nohash_hasher::IsEnabled>(
    current : &T,
    edges : &IntMap<T, &Vec<T>>,
    visited : &mut IntSet<T>,
    on_stack : &mut IntSet<T>,
    output : &mut Vec<T>
) -> bool {
    visited.insert(*current);
    on_stack.insert(*current);

    for adj in edges[&current] {
        if visited.contains(&adj) {
            if on_stack.contains(&adj) {
                return true;
            }
        } else {
            let nested_cyclic = dfs_impl::<T>(&adj, edges, visited, on_stack, output);
            if nested_cyclic {
                return true;
            }
        }
    }

    on_stack.remove(current);
    output.push(*current);
    false
}

#[cfg(test)]
mod test {
    #[test]
    pub fn kahn() {
        let sorted = super::TopologicalSorter::default()
            .add_node(0, vec![1, 2])
            .add_node(1, vec![])
            .add_node(2, vec![1, 3])
            .add_node(3, vec![])
            .add_node(4, vec![0])
            .sort_kahn();

        match sorted {
            Ok(sorted) => assert_eq!(sorted, vec![4, 0, 2, 3, 1]),
            Err(_) => panic!("Graph should not be cyclic"),
        };
    }

    #[test]
    pub fn depth_first() {
        let sorted = super::TopologicalSorter::default()
            .add_node(0, vec![1, 2])
            .add_node(1, vec![])
            .add_node(2, vec![1, 3])
            .add_node(3, vec![])
            .add_node(4, vec![0])
            .sort_dfs();

        match sorted {
            Ok(sorted) => assert_eq!(sorted, vec![4, 0, 2, 3, 1]),
            Err(_) => panic!(),
        }
    }
}