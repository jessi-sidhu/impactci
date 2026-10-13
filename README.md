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

## Roadmap

- [ ] week 1: parser (tree-sitter import resolution), tested on Flask and Requests
- [ ] week 2: dependency graph + `blast --diff main..HEAD`, JSON output
- [ ] week 3: test selection + safety fallbacks (dynamic imports, conftest, config changes)
- [ ] week 4: replay repo history, compare against testmon and full runs
- [ ] week 5: package the GitHub Action, PR comment, deploy on real repos
- [ ] week 6: real numbers, demo, Django benchmark
