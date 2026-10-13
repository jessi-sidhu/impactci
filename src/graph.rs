//! Module dependency graph. Edges point from importer to imported.

use std::collections::BTreeSet;

use petgraph::Direction;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::Bfs;
use petgraph::visit::Reversed;

use crate::resolve::{Project, Resolution};

pub struct Graph {
    graph: DiGraph<usize, ()>,
    nodes: Vec<NodeIndex>,
    /// Per module: imports that look internal but didn't resolve.
    pub unresolved: Vec<Vec<String>>,
}

impl Graph {
    pub fn build(project: &Project) -> Self {
        let mut graph = DiGraph::new();
        let nodes: Vec<_> = (0..project.modules.len())
            .map(|id| graph.add_node(id))
            .collect();
        let mut unresolved = vec![Vec::new(); project.modules.len()];

        for (id, m) in project.modules.iter().enumerate() {
            let mut targets = BTreeSet::new();
            for imp in &m.parsed.imports {
                match project.resolve(id, imp) {
                    Resolution::Internal { modules } => targets.extend(modules),
                    Resolution::External => {}
                    Resolution::Unresolved { reason } => unresolved[id].push(reason),
                }
            }
            // strings like "pkg.mod:obj" only count if they hit one of our modules;
            // a string that doesn't resolve is just a string
            for r in &m.parsed.string_refs {
                if let Resolution::Internal { modules } = project.resolve(id, r) {
                    targets.extend(modules);
                }
            }
            for t in targets {
                graph.add_edge(nodes[id], nodes[t], ());
            }
        }

        Graph {
            graph,
            nodes,
            unresolved,
        }
    }

    /// Everything that transitively imports any of `changed`, including `changed` itself.
    pub fn dependents(&self, changed: &[usize]) -> BTreeSet<usize> {
        let reversed = Reversed(&self.graph);
        let mut out = BTreeSet::new();
        for &c in changed {
            let mut bfs = Bfs::new(reversed, self.nodes[c]);
            while let Some(n) = bfs.next(reversed) {
                out.insert(self.graph[n]);
            }
        }
        out
    }

    /// Everything `module` transitively imports, including itself.
    pub fn dependencies(&self, module: usize) -> BTreeSet<usize> {
        let mut out = BTreeSet::new();
        let mut bfs = Bfs::new(&self.graph, self.nodes[module]);
        while let Some(n) = bfs.next(&self.graph) {
            out.insert(self.graph[n]);
        }
        out
    }

    /// How many modules import this one directly.
    pub fn fan_in(&self, module: usize) -> usize {
        self.graph
            .neighbors_directed(self.nodes[module], Direction::Incoming)
            .count()
    }

    pub fn direct_imports(&self, module: usize) -> BTreeSet<usize> {
        self.graph
            .neighbors_directed(self.nodes[module], Direction::Outgoing)
            .map(|n| self.graph[n])
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_source;
    use crate::resolve::Module;

    fn project(files: &[(&str, &str)]) -> Project {
        let modules = files
            .iter()
            .map(|(name, src)| Module {
                name: name.to_string(),
                path: format!("{}.py", name.replace('.', "/")).into(),
                is_package: false,
                parsed: parse_source(src).unwrap(),
            })
            .collect();
        Project::from_modules("/repo".into(), modules)
    }

    fn names(p: &Project, ids: BTreeSet<usize>) -> Vec<&str> {
        ids.into_iter()
            .map(|i| p.modules[i].name.as_str())
            .collect()
    }

    // a <- b <- c, d is on its own, e imports a
    fn sample() -> Project {
        project(&[
            ("a", ""),
            ("b", "import a\n"),
            ("c", "import b\n"),
            ("d", "import os\n"),
            ("e", "from a import thing\nimport missing_but_external\n"),
        ])
    }

    #[test]
    fn transitive_dependents() {
        let p = sample();
        let g = Graph::build(&p);
        assert_eq!(names(&p, g.dependents(&[0])), vec!["a", "b", "c", "e"]);
        assert_eq!(names(&p, g.dependents(&[1])), vec!["b", "c"]);
        assert_eq!(names(&p, g.dependents(&[3])), vec!["d"]);
    }

    #[test]
    fn transitive_dependencies() {
        let p = sample();
        let g = Graph::build(&p);
        assert_eq!(names(&p, g.dependencies(2)), vec!["a", "b", "c"]);
    }

    #[test]
    fn fan_in_counts_direct_importers() {
        let p = sample();
        let g = Graph::build(&p);
        assert_eq!(g.fan_in(0), 2);
        assert_eq!(g.fan_in(2), 0);
    }

    #[test]
    fn string_refs_add_edges() {
        let p = project(&[
            ("pkg", ""),
            ("pkg.plugins", ""),
            (
                "conftest",
                "pytest_plugins = ['pkg.plugins', 'other.thing']\n",
            ),
        ]);
        let g = Graph::build(&p);
        assert_eq!(
            names(&p, g.dependents(&[1])),
            vec!["pkg.plugins", "conftest"]
        );
        assert!(g.unresolved[2].is_empty());
    }

    #[test]
    fn cycles_dont_loop() {
        let p = project(&[("x", "import y\n"), ("y", "import x\n")]);
        let g = Graph::build(&p);
        assert_eq!(names(&p, g.dependents(&[0])), vec!["x", "y"]);
    }
}
