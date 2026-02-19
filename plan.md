Lets keep migrating from tokio to beet, which uses `async_` ecosystem, ie async_channel, async_io etc.

- remove `Method`, use beets HttpMethod


In general we should be leaning into beet/bevy paradigms, see beet examples for more info.

ie `../beet/examples/net/socket_client`

measure success by:
1. tokio is removed
2. `cargo run` actually spins up the server
use timeout, cos it will not return by itself.
