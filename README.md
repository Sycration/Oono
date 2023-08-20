## OONO
This project is an implementation of the Uno game, as a client-server application based on a distributed hashmap of games and a custom event loop. Written in 100% safe rust.

To run, simply run from the top-level directory (`oono/`)
`cargo run --release --bin oono` to run the client, or
`cargo run --release --bin oono-server` to run the server.

HTTPS may be added at a later date. Who cares if your uno game can be sniffed off the wire.