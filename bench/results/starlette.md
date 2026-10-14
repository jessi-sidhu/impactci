# starlette

30 commits replayed, 25 with planted bugs the full suite caught.

| | tests run | test time | full-suite runs | regressions missed |
|---|---|---|---|---|
| full suite | 100% | 100% | 30/30 | 0 |
| blast | 54% | 55% | 15/30 | 0/25 commits |
| blast --shallow-init | 54% | 55% | 15/30 | 0/25 commits |
| testmon (warm db) | 24% | 29% | 0/30 | 2/25 commits |

## Per commit

| commit | subject | mutated fns | regressions | blast | blast --shallow-init | testmon (warm db) |
|---|---|---|---|---|---|---|
| `7b2948fe7b` | Version 1.8.0 (#3660) | 0 | 0 | full | full | 0% |
| `5b544ced73` | Send response start immediately when GZipMiddleware cannot compress (#36 | 1 | 74 | 8% | 8% | 5% |
| `96d5023c55` | Emit `Max-Age=0` on session cookie when `max_age=0` (#3651) | 1 | 28 | 2% | 2% | 2% |
| `0a15da395d` | Include allowed methods in StaticFiles 405 responses (#3640) | 1 | 71 | 10% | 10% | 5% |
| `ff5b4df55a` | Close `Request.stream()` generators with `aclosing` (#3628) | 4 | 255 | full | full | 56% |
| `7cb2f09c18` | Mark sessions as accessed when popping existing keys (#3186) | 1 | 1 | 2% | 2% | 0% |
| `c67ca4eeef` | Add HTTP 103 Early Hints support (#3435) | 6 | 167 | full | full | 60% |
| `b2e8750b4a` | Bracket IPv6 hostnames in `URL.replace()` (#3617) | 1 | 32 | full | full | 68% |
| `48e657ab0c` | Resolve multipart charset before parsing fields (#3618) | 1 | 89 | full | full | 9% **missed 1** |
| `6beaa2e39d` | Preserve response trailers through GZipMiddleware (#3565) | 1 | 68 | 8% | 8% | 5% |
| `2284ff008c` | Stop file transfers when HTTP clients disconnect (#3523) | 6 | 164 | full | full | 69% |
| `85c3b7434f` | Ignore Range when If-Range contains a weak ETag (#3567) | 1 | 6 | full | full | 11% |
| `56d8858633` | Drop Python 3.10 support (#3592) | 0 | 0 | full | full | 92% |
| `9774b08ef4` | Stop multipart range responses on unexpected EOF (#3583) | 1 | 12 | full | full | 11% |
| `fada3631d8` | Make it possible to run the tests without blockbuster (#3576) | 0 | 0 | full | full | 100% |
| `2269e9a08c` | Version 1.7.0 (#3575) | 0 | 0 | full | full | 0% |
| `4fe55eb264` | Preserve `FileResponse` status for range requests (#3568) | 1 | 124 | full | full | 15% |
| `1f08daf162` | Mark OpenTelemetryMiddleware as experimental (#3574) | 0 | 0 | 4% | 4% | 4% |
| `57de5fa9c2` | Support HTTP response trailers in TestClient (#3563) | 2 | 1017 | full | full | 82% |
| `03f12b7fcf` | Allow HTTPException to use non-standard status codes (#3545) | 1 | 117 | full | full | 10% **missed 1** |
| `76fd00f1e2` | Reject `WebSocket` requests to `StaticFiles` (#3532) | 1 | 74 | 11% | 11% | 7% |
| `fd662b17b9` | Implement `identity` on `SimpleUser` and `UnauthenticatedUser` (#3271) | 2 | 4 | 1% | 1% | 1% |
| `3ebcaf38fb` | Run background tasks after the response is sent in `BaseHTTPMiddleware`  | 2 | 69 | 15% | 15% | 6% |
| `004250a3f2` | Inline `OpenTelemetryResponder` and document middleware parameters (#352 | 3 | 55 | 4% | 4% | 4% |
| `bc833498bb` | Handle standalone If-None-Match wildcards in StaticFiles (#3201) | 1 | 48 | 11% | 11% | 6% |
| `4ac5302dd4` | Detect blocking calls in coroutines using BlockBuster (#2858) | 1 | 435 | full | full | 84% |
| `8feafc6408` | Add provider parameters to OpenTelemetryMiddleware (#3520) | 3 | 55 | 4% | 4% | 4% |
| `b4c1d18161` | Handle empty and absent payloads in `WebSocketEndpoint.decode()` (#3372) | 1 | 16 | 8% | 8% | 2% |
| `0fcaff1d1e` | Accept collections in CORS configuration annotations (#3518) | 1 | 36 | 3% | 3% | 3% |
| `c9176b57af` | Vary CORS preflight responses by their request inputs (#3517) | 1 | 36 | 3% | 3% | 3% |

## Misses

- `48e657ab0c` testmon (warm db): tests/test_requests.py
- `03f12b7fcf` testmon (warm db): tests/test_routing.py

## Notes

- `48e657ab0c`: testmon skipped 1 failing tests, e.g. tests/test_requests.py::tests.test_requests::test_request_stream_then_body[trio]
- `03f12b7fcf`: testmon skipped 1 failing tests, e.g. tests/test_routing.py::tests.test_routing::test_mounted_middleware_does_not_catch_exception[trio]
