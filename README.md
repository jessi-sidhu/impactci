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

## GitHub Action

```yaml
on: pull_request

permissions:
  contents: read
  pull-requests: write   # for the comment

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0
      - uses: actions/setup-python@v5
        with:
          python-version: "3.12"
      - run: pip install -e . pytest
      - uses: jessi-sidhu/impactci@v0
        with:
          run: python -m pytest
```

The action diffs the PR against its base, posts (and keeps updating) one comment with
the blast radius, then runs your command on just the selected test files. It runs the
full command when blast decides it should, and skips it when nothing is affected.

| input | default | |
|---|---|---|
| `run` | | test command; selected files get appended |
| `path` | `.` | project root inside the repo |
| `base` | PR base | revision to diff against |
| `comment` | `true` | post the PR comment |
| `args` | | extra blast flags, e.g. `--shallow-init` |

Outputs: `mode` (`selected`/`full`), `tests`, `count`.

It downloads a prebuilt binary from this repo's releases when it can, and builds
from source (cached) otherwise. While the repo is private, other repos can only use it
if Actions access is granted in Settings → Actions → General → Access.

## CLI

```
cargo build --release

# which tests does this branch need?
./target/release/blast --diff main..HEAD path/to/repo

# machine readable
./target/release/blast --diff main..HEAD --json path/to/repo

# feed straight into pytest
pytest $(./target/release/blast --diff main..HEAD --format pytest-args)

# the PR comment
./target/release/blast --diff main..HEAD --format markdown

# why did this test get picked?
./target/release/blast why tests/test_views.py src/app/models.py

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
   Imports under `if TYPE_CHECKING:` are skipped, they never run.
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
resolve. `--trust-dynamic` turns off the first one. `import_module("x")` with a literal,
or inside `for x in ("a", "b"):`, is followed like a normal import.

`--shallow-init` is for libraries whose `__init__.py` re-exports everything. Normally
`from pkg.utils import x` depends on all of `pkg/__init__.py`'s imports, because Python
runs that file first. With the flag, only the `__init__.py` file itself counts. It can
miss a change that breaks things at import time, so it's opt in.

## Roadmap

- [x] week 1: parser (tree-sitter import resolution), tested on Flask and Requests
- [x] week 2: dependency graph + `blast --diff main..HEAD`, JSON output
- [x] week 3: test selection + safety fallbacks (dynamic imports, conftest, config changes)
- [ ] week 4: replay repo history, compare against testmon and full runs
- [ ] week 5: package the GitHub Action, PR comment, deploy on real repos
- [ ] week 6: real numbers, demo, Django benchmark
