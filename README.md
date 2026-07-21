# tower http utils

A small collection of utilities when working on HTTP services using tower.


## Type aliases

- `SvcBoxFut` shorthand for a pinned service future
- `Body` shorthand for an `UnsyncBoxBody` with `BoxError`, similar to axum's
body type
- `HyperService` type erased service, suitable for use in return position and
function arguments when working with hyper's `.serve_connection()`

## Helper functions
## Response body extension trait
