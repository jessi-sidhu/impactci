use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand, ValueEnum};
use serde::Serialize;

use blast::diff::{self, Change};
use blast::graph::Graph;
use blast::resolve::{Project, Resolution};
use blast::select::{self, Mode, Options, Selection};
use blast::{report, risk};

#[derive(Parser)]
#[command(
    name = "blast",
    version,
    about = "Figure out which Python tests a change can reach"
)]
#[command(args_conflicts_with_subcommands = true)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
    #[command(flatten)]
    run: RunArgs,
}

#[derive(Args)]
struct RunArgs {
    /// Git range to diff, e.g. main..HEAD
    #[arg(long, default_value = "origin/main..HEAD")]
    diff: String,
    /// Project root
    #[arg(default_value = ".")]
    path: PathBuf,
    #[arg(long, value_enum, default_value_t = Format::Text)]
    format: Format,
    /// Shorthand for --format json
    #[arg(long)]
    json: bool,
    /// Don't force tests that reach a non-literal dynamic import
    #[arg(long)]
    trust_dynamic: bool,
    /// Don't follow a package `__init__.py`'s re-exports when a test only imports a
    /// submodule. Selects far fewer tests in libraries, but can miss import-time breakage.
    #[arg(long)]
    shallow_init: bool,
    /// Also write the PR comment markdown to this file
    #[arg(long)]
    markdown_out: Option<PathBuf>,
}

#[derive(Clone, Copy, ValueEnum)]
enum Format {
    Text,
    Json,
    /// One test path per line, for `pytest $(blast --format pytest-args)`
    PytestArgs,
    /// The PR comment
    Markdown,
}

#[derive(Subcommand)]
enum Command {
    /// Print every module in the project and what it imports.
    Imports {
        /// Project root
        #[arg(default_value = ".")]
        path: PathBuf,
        /// Also list third-party / stdlib imports
        #[arg(long)]
        all: bool,
        #[arg(long)]
        json: bool,
    },
    /// Show the import chain that links a test (or any module) to another module.
    Why {
        /// Test or module path, e.g. tests/test_app.py
        from: PathBuf,
        /// Module path it supposedly depends on, e.g. src/pkg/models.py
        to: PathBuf,
        /// Project root
        #[arg(long, default_value = ".")]
        root: PathBuf,
        #[arg(long)]
        shallow_init: bool,
    },
}

#[derive(Serialize)]
struct ModuleImports {
    module: String,
    path: PathBuf,
    imports: Vec<String>,
    unresolved: Vec<String>,
    dynamic_imports: Vec<usize>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Some(Command::Imports { path, all, json }) => imports(path, all, json),
        Some(Command::Why {
            from,
            to,
            root,
            shallow_init,
        }) => why(&root, &from, &to, shallow_init),
        None => {
            let mut args = cli.run;
            if args.json {
                args.format = Format::Json;
            }
            run(args)
        }
    }
}

fn run(args: RunArgs) -> Result<()> {
    let project = Project::load(&args.path)?;
    let mut graph = Graph::build(&project);
    graph.shallow_init = args.shallow_init;
    let (changed, prefix) = changes_in_project(&args.path, &args.diff)?;
    let opts = Options {
        trust_dynamic: args.trust_dynamic,
    };
    let sel = select::select(&project, &graph, &changed, &opts);

    let wants_md = args.markdown_out.is_some() || matches!(args.format, Format::Markdown);
    let md = if wants_md {
        let churn = risk::churn(&args.path).unwrap_or_default();
        let paths: Vec<PathBuf> = changed.iter().map(|c| c.path.clone()).collect();
        let risks = risk::score(&project, &graph, &paths, &churn, &prefix);
        report::markdown(&sel, &risks)
    } else {
        String::new()
    };
    if let Some(out) = &args.markdown_out {
        std::fs::write(out, &md).with_context(|| format!("writing {}", out.display()))?;
    }

    match args.format {
        Format::Markdown => print!("{md}"),
        Format::Json => println!("{}", serde_json::to_string_pretty(&sel)?),
        Format::PytestArgs => {
            // full run: print nothing, bare `pytest` already runs everything
            if sel.mode == Mode::Selected {
                if sel.selected_tests.is_empty() {
                    eprintln!("blast: no tests affected");
                }
                for t in &sel.selected_tests {
                    println!("{}", t.display());
                }
            }
        }
        Format::Text => print_text(&sel),
    }
    Ok(())
}

fn print_text(sel: &Selection) {
    println!("{} changed files", sel.changed.len());
    for c in &sel.changed {
        println!("  {} ({:?})", c.path.display(), c.kind);
    }
    println!("{} affected modules", sel.affected_modules.len());
    for m in &sel.affected_modules {
        println!("  {m}");
    }
    match sel.mode {
        Mode::Full => println!("running the full suite ({} tests)", sel.total_tests),
        Mode::Selected => println!(
            "running {} of {} test files",
            sel.selected_tests.len(),
            sel.total_tests
        ),
    }
    for r in &sel.reasons {
        println!("  because {r}");
    }
    if sel.mode == Mode::Selected {
        for t in &sel.selected_tests {
            println!("  {}", t.display());
        }
    }
}

fn why(root: &Path, from: &Path, to: &Path, shallow_init: bool) -> Result<()> {
    let project = Project::load(root)?;
    let mut graph = Graph::build(&project);
    graph.shallow_init = shallow_init;
    let find = |p: &Path| {
        project
            .module_by_path(p)
            .with_context(|| format!("{} isn't a module under {}", p.display(), root.display()))
    };
    let (a, b) = (find(from)?, find(to)?);

    // tests also pick up everything their conftest.py files import
    let mut starts = vec![a];
    let mut dir = from.parent();
    while let Some(d) = dir {
        if let Some(c) = project.module_by_path(&d.join("conftest.py")) {
            starts.push(c);
        }
        dir = d.parent();
    }

    let best = starts
        .iter()
        .filter_map(|&s| graph.path(s, b))
        .min_by_key(Vec::len);
    match best {
        Some(chain) => {
            if chain[0] != a {
                println!("{} (via its conftest.py)", from.display());
            }
            for (i, m) in chain.iter().enumerate() {
                let m = &project.modules[*m];
                println!("{}{} ({})", "  ".repeat(i), m.name, m.path.display());
            }
        }
        None => println!("{} can't reach {}", from.display(), to.display()),
    }
    Ok(())
}

/// Changed files, with paths made relative to the project root (which may be a
/// subdirectory of the git repo). Files outside the project are dropped.
fn changes_in_project(project_root: &Path, range: &str) -> Result<(Vec<Change>, PathBuf)> {
    let root = project_root
        .canonicalize()
        .with_context(|| format!("can't open {}", project_root.display()))?;
    let repo = git2::Repository::discover(&root)?;
    let workdir = repo
        .workdir()
        .context("bare repos aren't supported")?
        .canonicalize()?;
    let prefix = root.strip_prefix(&workdir).unwrap_or(Path::new(""));

    let rel = |p: &Path| p.strip_prefix(prefix).ok().map(Path::to_path_buf);
    let changes = diff::changed_files(&root, range)?
        .into_iter()
        .filter_map(|c| {
            let path = rel(&c.path);
            let old_path = c.old_path.as_deref().and_then(rel);
            match (path, old_path) {
                (Some(path), old_path) => Some(Change {
                    path,
                    old_path,
                    kind: c.kind,
                }),
                // renamed out of the project: from here it looks like a deletion
                (None, Some(old)) => Some(Change {
                    path: old,
                    old_path: None,
                    kind: diff::ChangeKind::Deleted,
                }),
                (None, None) => None,
            }
        })
        .collect();
    Ok((changes, prefix.to_path_buf()))
}

fn imports(path: PathBuf, all: bool, json: bool) -> Result<()> {
    let project = Project::load(&path)?;
    let mut rows = Vec::new();

    for (id, m) in project.modules.iter().enumerate() {
        let mut imports = Vec::new();
        let mut unresolved = Vec::new();
        for imp in &m.parsed.imports {
            match project.resolve(id, imp) {
                Resolution::Internal { modules, .. } => {
                    for target in modules {
                        imports.push(project.modules[target].name.clone());
                    }
                }
                Resolution::External if all => imports.push(format!("{} (external)", imp.module)),
                Resolution::External => {}
                Resolution::Unresolved { reason } => unresolved.push(reason),
            }
        }
        imports.sort();
        imports.dedup();
        rows.push(ModuleImports {
            module: m.name.clone(),
            path: m.path.clone(),
            imports,
            unresolved,
            dynamic_imports: m.parsed.dynamic_imports.clone(),
        });
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&rows)?);
        return Ok(());
    }

    for r in &rows {
        println!("{} ({})", r.module, r.path.display());
        for i in &r.imports {
            println!("  -> {i}");
        }
        for u in &r.unresolved {
            println!("  ?? {u}");
        }
        for line in &r.dynamic_imports {
            println!("  !! dynamic import on line {line}");
        }
    }
    eprintln!("{} modules", rows.len());
    Ok(())
}
