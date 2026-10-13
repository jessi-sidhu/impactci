use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand};
use serde::Serialize;

use blast::resolve::{Project, Resolution};

#[derive(Parser)]
#[command(
    name = "blast",
    version,
    about = "Figure out which Python tests a change can reach"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
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
        Command::Imports { path, all, json } => imports(path, all, json),
    }
}

fn imports(path: PathBuf, all: bool, json: bool) -> Result<()> {
    let project = Project::load(&path)?;
    let mut rows = Vec::new();

    for (id, m) in project.modules.iter().enumerate() {
        let mut imports = Vec::new();
        let mut unresolved = Vec::new();
        for imp in &m.parsed.imports {
            match project.resolve(id, imp) {
                Resolution::Internal { modules } => {
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
