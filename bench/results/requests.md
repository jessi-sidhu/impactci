# requests

6 commits replayed, 5 with planted bugs the full suite caught.

| | tests run | test time | full-suite runs | regressions missed |
|---|---|---|---|---|
| full suite | 100% | 100% | 6/6 | 0 |
| blast | 100% | 100% | 6/6 | 0/5 commits |
| blast --shallow-init | 79% | 100% | 2/6 | 0/5 commits |
| testmon (warm db) | 35% | 53% | 0/6 | 0/5 commits |

## Per commit

| commit | subject | mutated fns | regressions | blast | blast --shallow-init | testmon (warm db) |
|---|---|---|---|---|---|---|
| `6f66281a1d` | Add hasattr checks for remaining protocol isinstance checks (#7505) | 4 | 273 | full | 95% | 57% |
| `6f205ff422` | Fix `_encode_files` detection for `__getattr__`-based file wrappers (#75 | 1 | 19 | full | 59% | 54% |
| `661970d171` | Disable commonly ignored Pyright linting rules (#7497) | 10 | 284 | full | full | 45% |
| `1190afd14f` | Add type annotation for `Request.hooks` (#7498) | 0 | 0 | full | 59% | 47% |
| `e50e594529` | Improve static typing of 3rd party imports  (#7496) | 3 | 22 | full | full | 4% |
| `cd90742ed9` | docs: indent continuation line in Session.get :param params: (#7460) | 1 | 29 | full | 59% | 5% |
