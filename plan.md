# Discord Bot — TODO

Lets keep iterating on our discord bot.

- still getting duplicates of each slash command, do we need to unregister all when we register, to reset the stale ones?


- after about 2 mins we get this disconnect with failed reconnect
```
2026-02-19T04:39:20.521889Z  WARN hello_discord::gateway: reconnecting after backoff delay_ms=2253 attempt=1
2026-02-19T04:39:22.776568Z  INFO hello_discord::gateway: connecting to Discord gateway url=wss://gateway-us-east1-c.discord.gg?v=10&encoding=json
2026-02-19T04:39:22.847713Z ERROR hello_discord::gateway: failed to connect to gateway error=HTTP error: 400 Bad Request
2026-02-19T04:39:22.847745Z  WARN hello_discord::gateway: backing off before reconnect delay_ms=4779 attempt=2
```



Once those are done we will enter a test loop. You will need good logs to understand what is happening.


1. start the server
2. I will asynchronously do some stuff, and then i will kill the server
3. read the logs and fix anything that needs fixing. if you dont understand what happened then add more logs and start the server again.
4. repeat until i say so.
