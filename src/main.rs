//! Discord bot entry point.
//!
//! All transport details live in `gateway` (WebSocket) and `http` (REST).
//! This file is purely bot logic: reacting to typed events.

use hello_discord::run;

fn main() {
    run();
}
