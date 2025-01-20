use std::collections::HashSet;
use crate::coordinates::Coordinate;
use crate::lockfile::Lockfile;

#[derive(Debug)]
pub struct DependencyPruner {
    marked: HashSet<Coordinate>,

    visited: HashSet<Coordinate>
}

impl DependencyPruner {
    pub fn new() -> Self {
        Self {
            marked: HashSet::new(),
            visited: HashSet::new(),
        }
    }

    pub(crate) fn mark_tree(&mut self, coord: &Coordinate, lockfile: &Lockfile) {
        if !self.visited.insert(coord.clone()) {
            return;
        }

        self.marked.insert(coord.clone());

        if let Some(pkg_info) = lockfile.deps.get(&coord.to_string()) {
            for dep_coord in &pkg_info.deps {
                if let Ok(dep) = Coordinate::parse(dep_coord) {
                    self.mark_tree(&dep, lockfile);
                }
            }
        }
    }

    pub fn get_removable(&self, lockfile: &Lockfile) -> HashSet<Coordinate> {
        lockfile.deps.keys()
            .filter_map(|coord_str| {
                let coord = Coordinate::parse(coord_str).ok()?;
                if self.marked.contains(&coord) {
                    Some(coord)
                } else {
                    None
                }
            })
            .collect()
    }
}