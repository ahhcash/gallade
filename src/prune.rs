use std::collections::HashSet;
use crate::coordinates::Coordinate;
use crate::lockfile::Lockfile;

#[derive(Debug)]
pub struct DependencyPruner {
    marked_from_removed: HashSet<Coordinate>,
    marked_from_remaining: HashSet<Coordinate>,
    visited: HashSet<Coordinate>
}

impl DependencyPruner {
    pub fn new() -> Self {
        Self {
            marked_from_removed: HashSet::new(),
            marked_from_remaining: HashSet::new(),
            visited: HashSet::new(),
        }
    }

    // dfs
    pub fn mark_tree(&mut self, coord: &Coordinate, lockfile: &Lockfile, for_removed: bool) {
        if !self.visited.insert(coord.clone()) {
            return;
        }

        if for_removed {
            self.marked_from_removed.insert(coord.clone());
        } else {
            self.marked_from_remaining.insert(coord.clone());
        }

        if let Some(pkg_info) = lockfile.deps.get(&coord.to_string()) {
            for dep_coord in &pkg_info.deps {
                if let Ok(dep) = Coordinate::parse(dep_coord) {
                    self.mark_tree(&dep, lockfile, for_removed);
                }
            }
        }
    }

    pub fn get_removable(&self) -> HashSet<Coordinate> {
        self.marked_from_removed
            .iter()
            .filter(|e| !self.marked_from_remaining.contains(&e))
            .cloned()
            .collect()
    }
}