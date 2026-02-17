# Discord Bot — TODO

Great first in building out our clean discord abstraction, lets keep iterating. Finish off these items.

- [ ] The registering and handling of commands should be more cohesive, we need a unified system instead of disjointed match statements.
- [ ] Send images/files. start with /send-logo: `./logo-square.png`
- [ ] Add select menu component for multi-choice interactions *(types.rs — string_select builder ready, needs a command to use it)*

- we now see duplicates of each slash command, if thats due to registering of guild specific ones to speed up the propagation, then surface a config for selecting known or all, noting that all will take longer to propagate.
- roll button is broken, it says received integer expected string


here are some logs from some clicking around i did
```
5466645
2026-02-17T05:01:38.491244Z  WARN hello_discord::events: failed to parse INTERACTION_CREATE payload event="INTERACTION_CREATE" error=invalid type: integer `2`, expected a string
2026-02-17T05:02:17.691039Z  WARN hello_discord::gateway: WebSocket closed by server close_code=Some(Normal)
2026-02-17T05:02:17.691080Z  INFO hello_discord::gateway: will attempt RESUME
2026-02-17T05:02:17.691092Z  WARN hello_discord::gateway: reconnecting after backoff delay_ms=2149 attempt=1
2026-02-17T05:02:19.841979Z  INFO hello_discord::gateway: connecting to Discord gateway url=wss://gateway-us-east1-b.discord.gg?v=10&encoding=json
2026-02-17T05:02:19.936663Z ERROR hello_discord::gateway: failed to connect to gateway error=HTTP error: 400 Bad Request
2026-02-17T05:02:19.936720Z  WARN hello_discord::gateway: backing off before reconnect delay_ms=4581 attempt=2
^[Y2026-02-17T05:02:24.518484Z  INFO hello_discord::gateway: connecting to Discord gateway url=wss://gateway-us-east1-b.discord.gg?v=10&encoding=json
2026-02-17T05:02:24.597465Z ERROR hello_discord::gateway: failed to connect to gateway error=HTTP error: 400 Bad Request
2026-02-17T05:02:24.597496Z  WARN hello_discord::gateway: backing off before reconnect delay_ms=8479 attempt=3
2026-02-17T05:02:33.077226Z  INFO hello_discord::gateway: connecting to Discord gateway url=wss://gateway-us-east1-b.discord.gg?v=10&encoding=json
2026-02-17T05:02:33.151760Z ERROR hello_discord::gateway: failed to connect to gateway error=HTTP error: 400 Bad Request
2026-02-17T05:02:33.151790Z  WARN hello_discord::gateway: backing off before reconnect delay_ms=14332 attempt=4
2026-02-17T05:02:47.484339Z  INFO hello_discord::gateway: connecting to Discord gateway url=wss://gateway-us-east1-b.discord.gg?v=10&encoding=json
2026-02-17T05:02:47.563942Z ERROR hello_discord::gateway: failed to connect to gateway error=HTTP error: 400 Bad Request
2026-02-17T05:02:47.563973Z  WARN hello_discord::gateway: backing off before reconnect delay_ms=36251 attempt=5
2026-02-17T05:03:23.815680Z  INFO hello_discord::gateway: connecting to Discord gateway url=wss://gateway-us-east1-b.discord.gg?v=10&encoding=json
2026-02-17T05:03:23.893775Z ERROR hello_discord::gateway: failed to connect to gateway error=HTTP error: 400 Bad Request
2026-02-17T05:03:23.893806Z  WARN hello_discord::gateway: backing off before reconnect delay_ms=60000 attempt=6
^[[A^[[B^[[A^[[B2026-02-17T05:04:23.894459Z  INFO hello_discord::gateway: connecting to Discord gateway url=wss://gateway-us-east1-b.discord.gg?v=10&encoding=json
2026-02-17T05:04:23.969933Z ERROR hello_discord::gateway: failed to connect to gateway error=HTTP error: 400 Bad Request
2026-02-17T05:04:23.969964Z  WARN hello_discord::gateway: backing off before reconnect delay_ms=60000 attempt=7
^C

```


Once those are done we will enter a test loop. You will need good logs to understand what is happening.


1. start the server
2. I will asynchronously do some stuff, and then i will kill the server
3. read the logs and fix anything that needs fixing. if you dont understand what happened then add more logs and start the server again.
4. repeat until i say so.
