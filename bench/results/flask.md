# flask

30 commits replayed, 6 with planted bugs the full suite caught.

| | tests run | test time | full-suite runs | regressions missed |
|---|---|---|---|---|
| full suite | 100% | 100% | 30/30 | 0 |
| blast | 92% | 88% | 26/30 | 0/6 commits |
| blast --shallow-init | 92% | 88% | 26/30 | 0/6 commits |
| testmon (warm db) | 60% | 64% | 0/30 | 0/6 commits |

## Per commit

| commit | subject | mutated fns | regressions | blast | blast --shallow-init | testmon (warm db) |
|---|---|---|---|---|---|---|
| `d086db856b` | fix typing | 0 | 0 | full | full | 88% |
| `329c32b916` | fix dev | 0 | 0 | full | full | 88% |
| `728c5e440a` | drop support for Python 3.10 (#6168) | 3 | 309 | full | full | 79% |
| `d73fa1cdcb` | fix docs ref | 2 | 277 | full | full | 56% |
| `d318b68347` | explain seek | 1 | 2 | full | full | 0% |
| `2a8a38b051` | support query in methodview | 0 | 0 | full | full | 5% |
| `d8eaaba824` | add `app.query` route decorator (#6133) | 1 | 1 | full | full | 88% |
| `05e9c6bd63` | Fix `.partition(":")` usage on potential IPv6 addresses (#6096) | 2 | 41 | full | full | 88% |
| `514fc6b3e8` | Merge branch 'stable' | 1 | 235 | full | full | 48% |
| `8b4fd5d186` | use header properties | 0 | 0 | 15% | 15% | 15% |
| `6a2f545bfd` | Merge branch 'stable' | 1 | 0 | full | full | 100% |
| `36e4a824f3` | Any for  CertParamType type | 0 | 0 | full | full | 100% |
| `954f5684e4` | update dev dependencies | 2 | 0 | full | full | - |
| `dcbede0cb0` | autoescape selection uses case-insensitive comparison (#6013) | 1 | 0 | full | full | 100% |
| `06ea505ce2` | separate copy per call | 2 | 0 | full | full | 100% |
| `258d68b6ff` | Merge branch 'stable' | 0 | 0 | 0% | 0% | 100% |
| `7ef2946fb5` | remove unicode host test (#5962) | 0 | 0 | 0% | 0% | 100% |
| `3a9d54f3da` | Merge branch 'stable' | 1 | 0 | full | full | 100% |
| `c34d6e81fd` | all teardown callbacks are called despite errors (#5928) | 7 | 0 | full | full | 100% |
| `7b0088693e` | fix typing | 1 | 0 | full | full | 100% |
| `a411a2434b` | add back opening session on context push | 3 | 0 | full | full | 100% |
| `daca74d93a` | Merge branch 'stable' | 5 | 0 | full | full | 100% |
| `d98eb69a35` | revert cli test change | 0 | 0 | 100% | 100% | 100% |
| `12e95c93b4` | fix provide_automatic_options override (#5917) | 1 | 0 | full | full | 100% |
| `d3b78fd18a` | Merge remote-tracking branch 'origin/stable' | 1 | 0 | full | full | 100% |
| `5e621a2801` | update domain matching tests for Werkzeug 3.2 | 0 | 0 | full | full | 100% |
| `4b8bde97d4` | deprecate `should_ignore_error` (#5899) | 2 | 0 | full | full | 100% |
| `5880befcd2` | Merge branch 'stable' | 3 | 0 | full | full | 100% |
| `809d5a8869` | redirect defaults to 303 (#5898) | 2 | 0 | full | full | 100% |
| `96a01e420b` | Merge branch 'stable' | 1 | 0 | full | full | 100% |

## Notes

- `728c5e440a`: testmon skipped 1 failing tests, e.g. tests/test_helpers.py::tests.test_helpers.TestHelpers::test_get_debug_flag[-False]
- `728c5e440a`: 1 missed tests pass on their own (order-dependent), not counted: tests/test_helpers.py::tests.test_helpers.TestHelpers::test_get_debug_flag[-False]
- `514fc6b3e8`: testmon skipped 1 failing tests, e.g. tests/test_helpers.py::tests.test_helpers.TestHelpers::test_get_debug_flag[-False]
- `514fc6b3e8`: 1 missed tests pass on their own (order-dependent), not counted: tests/test_helpers.py::tests.test_helpers.TestHelpers::test_get_debug_flag[-False]
- `6a2f545bfd`: mutants survived the full suite (untested code)
- `36e4a824f3`: 1 tests already failing before mutation
- `954f5684e4`: mutants survived the full suite (untested code)
- `dcbede0cb0`: 1 tests already failing before mutation
- `dcbede0cb0`: mutants survived the full suite (untested code)
- `06ea505ce2`: 1 tests already failing before mutation
- `06ea505ce2`: mutants survived the full suite (untested code)
- `258d68b6ff`: 1 tests already failing before mutation
- `7ef2946fb5`: 1 tests already failing before mutation
- `3a9d54f3da`: 1 tests already failing before mutation
- `3a9d54f3da`: mutants survived the full suite (untested code)
- `c34d6e81fd`: 1 tests already failing before mutation
- `c34d6e81fd`: mutants survived the full suite (untested code)
- `7b0088693e`: 1 tests already failing before mutation
- `7b0088693e`: mutants survived the full suite (untested code)
- `a411a2434b`: 1 tests already failing before mutation
- `a411a2434b`: mutants survived the full suite (untested code)
- `daca74d93a`: 1 tests already failing before mutation
- `daca74d93a`: mutants survived the full suite (untested code)
- `d98eb69a35`: 1 tests already failing before mutation
- `12e95c93b4`: 1 tests already failing before mutation
- `12e95c93b4`: mutants survived the full suite (untested code)
- `d3b78fd18a`: 1 tests already failing before mutation
- `d3b78fd18a`: mutants survived the full suite (untested code)
- `5e621a2801`: 1 tests already failing before mutation
- `4b8bde97d4`: 1 tests already failing before mutation
- `4b8bde97d4`: mutants survived the full suite (untested code)
- `5880befcd2`: 1 tests already failing before mutation
- `5880befcd2`: mutants survived the full suite (untested code)
- `809d5a8869`: 1 tests already failing before mutation
- `809d5a8869`: mutants survived the full suite (untested code)
- `96a01e420b`: 1 tests already failing before mutation
- `96a01e420b`: mutants survived the full suite (untested code)
