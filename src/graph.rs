//! Module dependency graph. Edges point from importer to imported.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use petgraph::Direction;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;

use crate::resolve::{Project, Resolution};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Edge {
    /// Actually uses the module.
    Import,
    /// Only runs the module because it's a parent package's `__init__.py`.
    Parent,
}

pub struct Graph {
    graph: DiGraph<usize, Edge>,
    nodes: Vec<NodeIndex>,
    /// Per module: imports that look internal but didn't resolve.
    pub unresolved: Vec<Vec<String>>,
    /// Don't follow a package `__init__.py`'s own imports when it was only reached as a
    /// parent. Big win for libraries whose `__init__` re-exports everything, but it can
    /// miss import-time breakage, so it's opt in.
    pub shallow_init: bool,
}

impl Graph {
    pub fn build(project: &Project) -> Self {
        let mut graph = DiGraph::new();
        let nodes: Vec<_> = (0..project.modules.len())
            .map(|id| graph.add_node(id))
            .collect();
        let mut unresolved = vec![Vec::new(); project.modules.len()];

        for (id, m) in project.modules.iter().enumerate() {
            // Import beats Parent if a module is reached both ways
            let mut targets: BTreeMap<usize, Edge> = BTreeMap::new();
            let mut add = |res: Resolution| {
                if let Resolution::Internal { modules, parents } = res {
                    for t in modules {
                        let kind = if parents.contains(&t) {
                            Edge::Parent
                        } else {
                            Edge::Import
                        };
                        let e = targets.entry(t).or_insert(kind);
                        *e = (*e).min(kind);
                    }
                }
            };
            for imp in &m.parsed.imports {
                match project.resolve(id, imp) {
                    Resolution::Unresolved { reason } => unresolved[id].push(reason),
                    res => add(res),
                }
            }
            // strings like "pkg.mod:obj" only count if they hit one of our modules;
            // a string that doesn't resolve is just a string
            for r in &m.parsed.string_refs {
                add(project.resolve(id, r));
            }
            for (t, kind) in targets {
                graph.add_edge(nodes[id], nodes[t], kind);
            }
        }

        Graph {
            graph,
            nodes,
            unresolved,
            shallow_init: false,
        }
    }

    fn follows(&self, kind: Edge) -> bool {
        kind == Edge::Import || !self.shallow_init
    }

    /// Everything whose code can end up running something in `changed`, including
    /// `changed` itself.
    pub fn dependents(&self, changed: &[usize]) -> BTreeSet<usize> {
        let changed_set: BTreeSet<usize> = changed.iter().copied().collect();
        let mut out = BTreeSet::new();
        let mut queue: VecDeque<usize> = changed.iter().copied().collect();
        out.extend(changed.iter().copied());

        while let Some(m) = queue.pop_front() {
            for e in self
                .graph
                .edges_directed(self.nodes[m], Direction::Incoming)
            {
                // reaching a changed module as a parent still runs it; reaching a module
                // that merely imports the change as a parent doesn't
                if !changed_set.contains(&m) && !self.follows(*e.weight()) {
                    continue;
                }
                let importer = self.graph[e.source()];
                if out.insert(importer) {
                    queue.push_back(importer);
                }
            }
        }
        out
    }

    /// Everything `module` can end up running, including itself.
    pub fn dependencies(&self, module: usize) -> BTreeSet<usize> {
        let mut out = BTreeSet::from([module]);
        let mut expanded = BTreeSet::from([module]);
        let mut queue = VecDeque::from([module]);

        while let Some(m) = queue.pop_front() {
            for e in self
                .graph
                .edges_directed(self.nodes[m], Direction::Outgoing)
            {
                let target = self.graph[e.target()];
                out.insert(target);
                if self.follows(*e.weight()) && expanded.insert(target) {
                    queue.push_back(target);
                }
            }
        }
        out
    }

    /// Shortest import chain from `from` to `to`, both ends included.
    pub fn path(&self, from: usize, to: usize) -> Option<Vec<usize>> {
        let mut prev: BTreeMap<usize, usize> = BTreeMap::new();
        let mut expanded = BTreeSet::from([from]);
        let mut queue = VecDeque::from([from]);
        while let Some(m) = queue.pop_front() {
            if m == to {
                let mut chain = vec![to];
                let mut cur = to;
                while let Some(&p) = prev.get(&cur) {
                    chain.push(p);
                    cur = p;
                }
                chain.reverse();
                return Some(chain);
            }
            for e in self
                .graph
                .edges_directed(self.nodes[m], Direction::Outgoing)
            {
                let target = self.graph[e.target()];
                let can_expand = self.follows(*e.weight()) || target == to;
                if can_expand && expanded.insert(target) {
                    prev.insert(target, m);
                    queue.push_back(target);
                }
            }
        }
        None
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
    fn shallow_init_skips_reexports() {
        // pkg/__init__ re-exports a and b; t only uses pkg.a
        let p = Project::from_modules(
            "/repo".into(),
            vec![
                Module {
                    name: "pkg".into(),
                    path: "pkg/__init__.py".into(),
                    is_package: true,
                    parsed: parse_source("from .a import x\nfrom .b import y\n").unwrap(),
                },
                Module {
                    name: "pkg.a".into(),
                    path: "pkg/a.py".into(),
                    is_package: false,
                    parsed: parse_source("").unwrap(),
                },
                Module {
                    name: "pkg.b".into(),
                    path: "pkg/b.py".into(),
                    is_package: false,
                    parsed: parse_source("").unwrap(),
                },
                Module {
                    name: "t".into(),
                    path: "t.py".into(),
                    is_package: false,
                    parsed: parse_source("from pkg.a import x\n").unwrap(),
                },
                Module {
                    name: "u".into(),
                    path: "u.py".into(),
                    is_package: false,
                    parsed: parse_source("import pkg\n").unwrap(),
                },
            ],
        );
        let mut g = Graph::build(&p);
        assert_eq!(
            names(&p, g.dependents(&[2])),
            vec!["pkg", "pkg.b", "t", "u"]
        );

        g.shallow_init = true;
        assert_eq!(names(&p, g.dependents(&[2])), vec!["pkg", "pkg.b", "u"]);
        assert_eq!(names(&p, g.dependencies(3)), vec!["pkg", "pkg.a", "t"]);
        // the __init__ itself changing still reaches t
        assert_eq!(names(&p, g.dependents(&[0])), vec!["pkg", "t", "u"]);
    }

    #[test]
    fn cycles_dont_loop() {
        let p = project(&[("x", "import y\n"), ("y", "import x\n")]);
        let g = Graph::build(&p);
        assert_eq!(names(&p, g.dependents(&[0])), vec!["x", "y"]);
    }
}
