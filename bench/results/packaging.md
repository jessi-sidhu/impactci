# packaging

testmon is left out here. packaging is one of pytest's own dependencies, so pytest imports it before testmon starts tracing. With packaging's pytest config (`-p no:legacypath`) testmon 2.2.0 crashes outright, and with that worked around it reran all 62k tests on the commit we tried. Neither is a fair comparison.

20 commits replayed, 10 with planted bugs the full suite caught.

| | tests run | test time | full-suite runs | regressions missed |
|---|---|---|---|---|
| full suite | 100% | 100% | 20/20 | 0 |
| blast | 61% | 63% | 12/20 | 0/10 commits |
| blast --shallow-init | 61% | 63% | 12/20 | 0/10 commits |

## Per commit

| commit | subject | mutated fns | regressions | blast | blast --shallow-init |
|---|---|---|---|---|---|
| `ef837aad31` | test: avoid filtering compatible release examples (#1407) | 2 | 0 | 0% | 0% |
| `ae9d39c13b` | fix: require the complete pylock filename to match (#1446) | 1 | 5 | 0% | 0% |
| `332d716676` | refactor(version): use explicit keywords in __replace__ (#1378) | 1 | 1 | full | full |
| `fa52d7237a` | Fix pylock filenames with mixed Windows path separators (#1436) | 1 | 37 | 0% | 0% |
| `2aada00b15` | fix: reject empty arbitrary versions in requirements (#1430) | 0 | 0 | full | full |
| `2c87a33f66` | fix(licenses): canonicalize_license_expression mangling case-distinct Li | 1 | 70 | 1% | 1% |
| `0625596e50` | fix: anchor the wheel build tag regex to the end of the part (#1427) | 0 | 0 | full | full |
| `959bb79c06` | fix(dependency-groups): collect errors from sibling includes after a fir | 3 | 37 | 0% | 0% |
| `d17cfaf0b7` | perf(parser): avoid literal_eval for plain marker strings (#1429) | 1 | 1 | full | full |
| `f7c930eb18` | docs: sphinx.ext.autodoc for pylock and requirements (#1393) | 1 | 5359 | 9% | 9% |
| `022ae6c6fe` | fix: reject drive-relative license-file paths in metadata validation (#1 | 1 | 13 | 1% | 1% |
| `2602dd07d8` | fix(parser): reject whitespace in the URL token (#1379) | 0 | 0 | full | full |
| `23669b1b69` | feat(pylock.select): fail on unknown extras and dependency groups (#1376 | 1 | 32 | 0% | 0% |
| `9483cb8a74` | perf(markers): speed up _format_marker and benchmark str(Marker) (#1371) | 2 | 3985 | full | full |
| `de7d962e83` | ci(downstream): bump twine to 7.0.0, deselect readme_renderer 46 test (# | 0 | 0 | full | full |
| `b9d249f585` | perf(tests): disable more unused pytest plugins (#1390) | 1 | 0 | full | full |
| `053c884615` | tags: add interpreter_abi (#611) | 0 | 0 | full | full |
| `55cbf1b942` | fix(ranges): stop unbounded bounds sorting above each other (#1392) | 2 | 0 | full | full |
| `4840c3a681` | perf(tests): skip coverage on 3.13t (#1391) | 1 | 0 | full | full |
| `56dd80bca9` | perf(tests): disable GC while collecting tests (#1389) | 0 | 0 | full | full |

## Notes

- `ef837aad31`: mutants survived the full suite (untested code)
- `b9d249f585`: mutants survived the full suite (untested code)
- `55cbf1b942`: mutants survived the full suite (untested code)
- `4840c3a681`: mutants survived the full suite (untested code)
