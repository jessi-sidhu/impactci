# Trying blast on your repo

Thanks for trying this out. It takes about five minutes, and for the first couple of
weeks **nothing gets skipped**: blast runs in shadow mode, your full suite still runs,
and the PR comment shows what it *would* have done.

## 1. Add the workflow

`.github/workflows/blast.yml`:

```yaml
name: blast

on: pull_request

permissions:
  contents: read
  pull-requests: write

jobs:
  tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0
      - uses: actions/setup-python@v5
        with:
          python-version: "3.12"
      # whatever you already do to install your project and test deps
      - run: pip install -e . pytest
      - uses: jessi-sidhu/impactci@v0
        with:
          run: python -m pytest
          shadow: "true"
```

If your project isn't at the repo root, add `path: backend` (or wherever it lives).
If you already have a test workflow, you can run this one next to it.

## 2. Open PRs like normal

Each PR gets one comment (updated on every push) with:

- how many test files blast would run, and why
- the riskiest changed files (lots of modules depend on them and they change often)
- in shadow mode: test time it would have saved, and whether any failing test was
  outside its selection

If a comment ever says **"it would have missed failures in ..."**, please send me the
PR link. That's a bug, and exactly what shadow mode is there to catch.

## 3. Switch it on

Once you trust it, drop `shadow: "true"`. From then on it runs only the selected tests,
and the full suite whenever it's unsure (config changes, `conftest.py`, data files,
anything it can't follow statically).

A good middle ground is to keep a nightly or merge-to-main job that runs everything.

## Knobs

| input | |
|---|---|
| `args: --shallow-init` | for libraries whose `__init__.py` re-exports everything, much smaller selections, small risk around import-time errors |
| `args: --trust-dynamic` | don't widen the selection for `importlib.import_module(variable)` |
| `args: --python-files tests.py` | override test file patterns (normally read from your pytest config) |
| `comment: "false"` | no PR comment |

## Seeing why a test was (or wasn't) picked

```
cargo install --git https://github.com/jessi-sidhu/impactci
blast --diff origin/main..HEAD
blast why tests/test_orders.py app/billing/tax.py
```

## Access

The repo is private for now. If you're in a different org/account, I need to add you,
or flip it to public, before `uses: jessi-sidhu/impactci@v0` will resolve.
