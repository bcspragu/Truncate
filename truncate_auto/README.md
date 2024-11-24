# Truncate Auto

`truncate_auto` is a CLI for playing a game of Truncate over gRPC. The general expectation is that it's two Truncate-playing programs, but since error-handling is included and the bi-di protocol can tolerate long delays, it could be hooked up to human players as well.

## Implementing the API

The API is defined as a gRPC service in `proto/service.proto`. It's currently just a single `PlayGame` endpoint that operates as a bi-directional stream. The expected flow is as follows:

### Initialization

1. Client connects to the `PlayGame` endpoint
1. Client sends an `InitRequest` containing their name
1. Server waits until a second client is available to play
1. Server sends an `InitReply` to each players
  - This contains their player ID, initial tiles, initial board, and opponents

### Main game loop

1. Server sends a `MoveSolicitation` to one player
  - Starting with the first player who connected (for now)
  - This contains the latest board state
1. Server waits for a `MoveRequest` from the client
  - If other clients erroneously send requests, they'll be queued, and will likely receive `ErrorReply` responses
1. Client sends a `MoveRequest` containing their move
  - E.g. 'place tile Z at (x, y)' or 'swap tiles at (w, x) and (y, z)'
1. If the move is valid, the server sends back a `MoveReply`
  - This contains the player's new hand, the updated board, and if the game is over
  - If the move is invalid, the user will get an `ErrorReply`
    - In this case, they'll receive another `MoveSolicitation`

### Other API details

Clients can specify a `request_id` in their requests. For any `*Reply` type (`InitReply`, `MoveReply`, `ErrorReply`), the response will mirror back that `request_id`. This shouldn't _really_ be needed as the sequence of events should always be the same, but some clients might find it useful.

## `testclient`

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

- [~] Refactor the code to make it less hideous
  - In progress!
- [ ] Handle end-games
- [ ] Handle error cases better
- [ ] Audit gRPC API
- [ ] Add other functionality like time limits and custom board shapes
- [ ] Only allow a certain (1? 2? 3?) invalid moves in a row from a single player before ending the game
- [ ] Clean-up connection-closing/mpsc-closing behavior
