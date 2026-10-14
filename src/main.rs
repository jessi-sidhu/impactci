use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand, ValueEnum};
use serde::Serialize;

use blast::diff::{self, Change};
use blast::graph::Graph;
use blast::resolve::{Project, Resolution};
use blast::select::{self, Mode, Options, Selection};

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
}

#[derive(Clone, Copy, ValueEnum)]
enum Format {
    Text,
    Json,
    /// One test path per line, for `pytest $(blast --format pytest-args)`
    PytestArgs,
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
    let graph = Graph::build(&project);
    let changed = changes_in_project(&args.path, &args.diff)?;
    let opts = Options {
        trust_dynamic: args.trust_dynamic,
    };
    let sel = select::select(&project, &graph, &changed, &opts);

    match args.format {
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

/// Changed files, with paths made relative to the project root (which may be a
/// subdirectory of the git repo). Files outside the project are dropped.
fn changes_in_project(project_root: &Path, range: &str) -> Result<Vec<Change>> {
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
    Ok(diff::changed_files(&root, range)?
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
        .collect())
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
