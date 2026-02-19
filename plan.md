# Discord Bot — TODO

Lets keep iterating on our discord bot.

## Duplicate slash commands

- still getting duplicates of each slash command, do we need to unregister all when we register, to reset the stale ones that are currently registered?

## Disconnects

- after about 2 minutes we get this disconnect with failed reconnect. are we doing the heartbeat correctly?
and why is the reconnect failing?
```sh
2026-02-19T09:43:28.267073Z  WARN hello_discord::gateway: WebSocket closed by server close_code=Some(Normal)
2026-02-19T09:43:28.267116Z  INFO hello_discord::gateway: will attempt RESUME
2026-02-19T09:43:28.267148Z  WARN hello_discord::gateway: reconnecting after backoff delay_ms=1972 attempt=1
2026-02-19T09:43:30.240615Z  INFO hello_discord::gateway: connecting to Discord gateway url=wss://gateway-us-east1-b.discord.gg?v=10&encoding=json
2026-02-19T09:43:30.327417Z ERROR hello_discord::gateway: failed to connect to gateway error=HTTP error: 400 Bad Request
```
