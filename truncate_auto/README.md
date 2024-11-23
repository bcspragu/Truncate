# Truncate Auto

`truncate_auto` is a CLI for playing a game of Truncate over gRPC. The general expectation is that it's two Truncate-playing programs, but since error-handling is included, it could be hooked up to human players as well.

The `testclient` client is hooked up to the Truncate-included AI from the `npc` package.

To test it out:

```bash
# Run the server
cargo run --bin truncate_auto

# In two new terminals, run two clients
cargo run --bin testclient
cargo run --bin testclient
```

Once the second test client connects, the game will start playing automatically.

## TODO

- [ ] Refactor the code to make it less hideous
- [ ] Handle end-games
- [ ] Handle error cases better
- [ ] Audit gRPC API
- [ ] Add other functionality like time limits and custom board shapes
