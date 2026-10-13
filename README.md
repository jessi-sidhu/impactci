# impactci

Run only the Python tests your PR can actually affect.

## The problem

Most Python teams run the full test suite on every PR, even for a one-line change.
CI takes 20–40 minutes, people sit around waiting, and the CI bill keeps growing.

The existing options don't fit small and mid-size teams very well:

- **pytest-testmon** needs coverage data saved from earlier runs, which is awkward in fresh CI containers.
- **Bazel** means rewriting your whole build.
- **Launchable** is paid.

## The idea

A zero-setup GitHub Action. It reads the PR diff, statically maps which modules and
tests the change can reach, runs only those tests, and comments the blast radius on the PR.

- stateless — no coverage database, nothing to cache between runs
- one line of YAML
- conservative — when it's unsure it runs more tests, never fewer

## Usage (so far)

```
cargo build --release

# which tests does this branch need?
./target/release/blast --diff main..HEAD path/to/repo

# machine readable
./target/release/blast --diff main..HEAD --json path/to/repo

# feed straight into pytest
pytest $(./target/release/blast --diff main..HEAD --format pytest-args)

# debug: what does each module import?
./target/release/blast imports path/to/repo
```

`--diff` always compares against the merge base, same as a GitHub PR.

With `--format pytest-args`, a full run prints nothing (so bare `pytest` runs
everything). If no tests are affected it also prints nothing and says so on stderr,
so check for that before calling pytest if you want to skip the run.

## How it decides

1. Parse every `.py` file with tree-sitter and resolve imports (absolute, relative,
   `src/` layout, namespace subpackages). Importing `a.b.c` also counts as importing
   `a` and `a.b`, since their `__init__.py` runs.
2. Strings that name one of your modules (`pytest_plugins`, `"pkg.mod:obj"`,
   `import_module("pkg.mod")`) count as imports too.
3. Walk the graph backwards from the changed files. A test is selected if it, or any
   `conftest.py` above it, can reach a changed module.

It runs the **full suite** when:

- a `conftest.py` changes
- build/test config changes (`pyproject.toml`, `setup.cfg`, `tox.ini`, `requirements*.txt`, CI workflows, ...)
- a non-Python file changes that isn't obviously docs (templates, json fixtures, ...)
- a top-level module is deleted
- every test ends up selected anyway

It also **adds** tests when a module they reach does something it can't follow:
`import_module(name)` with a variable, or an import that looks internal but doesn't
resolve. `--trust-dynamic` turns off the first one.

## Roadmap

- [x] week 1: parser (tree-sitter import resolution), tested on Flask and Requests
- [x] week 2: dependency graph + `blast --diff main..HEAD`, JSON output
- [x] week 3: test selection + safety fallbacks (dynamic imports, conftest, config changes)
- [ ] week 4: replay repo history, compare against testmon and full runs
- [ ] week 5: package the GitHub Action, PR comment, deploy on real repos
- [ ] week 6: real numbers, demo, Django benchmark
