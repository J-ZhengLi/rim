//! This module defines **trait** that includes methods to handle package dependency,
//! like for `topological sorting` etc. Not the actual structure itself.

use indexmap::IndexMap;
use rim_common::types::ToolInfo;
use std::collections::{HashMap, VecDeque};

use super::tools::{Tool, ToolWithDeps};

/// A trait to support dependency handling, such as sorting.
pub trait DependencyHandler<T> {
    /// Perform topological sorting using [`Kahn's algorithm`][kahn],
    /// and return the sorted result in a `Vec`.
    ///
    /// This will place the item that have most dependencies at front, and the item
    /// with least dependencies at the end.
    ///
    /// [kahn]: https://en.wikipedia.org/wiki/Topological_sorting#Kahn's_algorithm
    /// [toolmap]: rim_common::types::ToolMap
    fn topological_sorted(&self) -> Vec<T>;

    /// Perform basic sorting for dependency handling, then return the result in a new `Vec`.
    ///
    /// For example, sorting the package installation order by its type.
    /// This should be used as a fallback to `topological_sorted`.
    ///
    /// Note: The default implementation is just a wrapper of
    /// [`topological_sorted`](DependencyHandler::topological_sorted).
    fn sorted(&self) -> Vec<T> {
        self.topological_sorted()
    }
}

impl<'a> DependencyHandler<(&'a str, &'a ToolInfo)> for Vec<(&'a str, &'a ToolInfo)> {
    fn topological_sorted(&self) -> Vec<(&'a str, &'a ToolInfo)> {
        // first, we initialize a hashmap that representing a graph
        let mut graph = HashMap::new();
        // then, we initialize a map to keep track of the 'depth' of each node.
        // use `IndexMap` to preserve original order
        let mut with_depths: IndexMap<&str, u32> = IndexMap::new();

        for (name, info) in self {
            let dependencies = info.dependencies();
            graph.insert(*name, (dependencies, *info));
            with_depths.entry(name).or_insert(0);
            for dep in dependencies {
                *with_depths.entry(dep).or_insert(0) += 1;
            }
        }

        let mut res = Vec::new();
        // all nodes with no incoming edge
        let mut queue = with_depths
            .iter()
            .filter_map(|(name, depth)| (*depth == 0).then_some(*name))
            .collect::<VecDeque<_>>();

        while let Some(name) = queue.pop_front() {
            let (deps, node) = graph.get(name).unwrap_or_else(|| {
                unreachable!("`{name}` in queue came from the graph, so this unwrap has no issue")
            });

            res.push((name, *node));
            for dep in *deps {
                let Some(depth) = with_depths.get_mut(dep.as_str()) else {
                    continue;
                };
                *depth = depth.saturating_sub(1);
                if *depth == 0 {
                    queue.push_back(dep);
                }
            }
        }

        res
    }
}

// TODO: Refractor duplicated code
impl<'a> DependencyHandler<Tool<'a>> for Vec<ToolWithDeps<'a>> {
    fn topological_sorted(&self) -> Vec<Tool<'a>> {
        // first, we initialize a hashmap that representing a graph
        let mut graph = HashMap::new();
        // then, we initialize a map to keep track of the 'depth' of each node.
        // use `IndexMap` to preserve original order
        let mut with_depths: IndexMap<&str, u32> = IndexMap::new();

        for tool_with_deps in self {
            let name = tool_with_deps.tool.name();
            let dependencies = tool_with_deps.dependencies;
            graph.insert(name, (dependencies, &tool_with_deps.tool));
            with_depths.entry(name).or_insert(0);
            for dep in dependencies {
                *with_depths.entry(dep).or_insert(0) += 1;
            }
        }

        let mut res = Vec::new();
        // all nodes with no incoming edge
        let mut queue = with_depths
            .iter()
            .filter_map(|(name, depth)| (*depth == 0).then_some(*name))
            .collect::<VecDeque<_>>();

        while let Some(name) = queue.pop_front() {
            let (deps, node) = graph.get(name).copied().unwrap_or_else(|| {
                unreachable!("`{name}` in queue came from the graph, so this unwrap has no issue")
            });

            res.push(node.clone());
            for dep in deps {
                let Some(depth) = with_depths.get_mut(dep.as_str()) else {
                    continue;
                };
                *depth = depth.saturating_sub(1);
                if *depth == 0 {
                    queue.push_back(dep);
                }
            }
        }

        res
    }

    fn sorted(&self) -> Vec<Tool<'a>> {
        let mut tools = self.iter().map(|t| t.tool.clone()).collect::<Vec<_>>();
        tools.sort_by(|a, b| a.kind.cmp(&b.kind));
        tools
    }
}

#[cfg(test)]
mod tests {
    use rim_common::types::ToolInfoDetails;

    use super::*;

    #[test]
    fn chain_dependencies_sorting() {
        let tools: Vec<(&str, ToolInfo)> = vec![
            (
                "a",
                ToolInfo::Complex(Box::new(ToolInfoDetails {
                    requires: vec!["b".to_string()],
                    ..Default::default()
                })),
            ),
            ("c", ToolInfo::Complex(Box::new(ToolInfoDetails::default()))),
            (
                "b",
                ToolInfo::Complex(Box::new(ToolInfoDetails {
                    requires: vec!["c".to_string()],
                    ..Default::default()
                })),
            ),
            (
                "d",
                ToolInfo::Complex(Box::new(ToolInfoDetails {
                    requires: vec!["a".to_string()],
                    ..Default::default()
                })),
            ),
        ];

        let sorted = tools
            .iter()
            .map(|(name, info)| (*name, info))
            .collect::<Vec<_>>()
            .topological_sorted();

        let mut iter = sorted.iter();
        assert_eq!(iter.next().unwrap().0, "d");
        assert_eq!(iter.next().unwrap().0, "a");
        assert_eq!(iter.next().unwrap().0, "b");
        assert_eq!(iter.next().unwrap().0, "c");
    }

    #[test]
    fn no_dependencies_sorting() {
        let tools: Vec<(&str, ToolInfo)> = vec![
            ("a", ToolInfo::Complex(Box::new(ToolInfoDetails::default()))),
            ("c", ToolInfo::Complex(Box::new(ToolInfoDetails::default()))),
            ("b", ToolInfo::Complex(Box::new(ToolInfoDetails::default()))),
            ("d", ToolInfo::Complex(Box::new(ToolInfoDetails::default()))),
        ];

        let sorted = tools
            .iter()
            .map(|(name, info)| (*name, info))
            .collect::<Vec<_>>()
            .topological_sorted();

        let mut iter = sorted.iter();
        assert_eq!(iter.next().unwrap().0, "a");
        assert_eq!(iter.next().unwrap().0, "c");
        assert_eq!(iter.next().unwrap().0, "b");
        assert_eq!(iter.next().unwrap().0, "d");
    }
}
