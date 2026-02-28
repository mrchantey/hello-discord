//! Discord bot entry point.
//!
//! All transport details live in `discord_io/gateway` (WebSocket) and
//! `discord_io/http` (REST). This file is purely bot logic: reacting to
//! typed events.

#[cfg(feature = "io")]
use hello_discord::run;

fn main() {
    #[cfg(feature = "io")]
    run();

    #[cfg(not(feature = "io"))]
    eprintln!("Built without the `io` feature — nothing to run. Re-build with `--features io`.");
}
