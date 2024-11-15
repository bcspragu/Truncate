# Truncate Auto

`truncate_auto` is a CLI for playing a game of Truncate over gRPC. The general expectation is that it's two Truncate-playing programs, but since error-handling is included, it could be hooked up to human players as well.

The `testclient` client is hooked up to the Truncate-included AI from the `npc` package. Note that it doesn't actually work yet, something about setting up the local game state is broken, fails with the error:

```
thread 'tokio-runtime-worker' panicked at truncate_core/src/npc/mod.rs:139:13:
Expected a valid position to be playable
```
