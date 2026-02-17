# Discord Bot - Direct WebSocket API (Rust)

A minimal Discord bot using Rust that directly implements Discord's WebSocket Gateway protocol without any Discord-specific libraries.

## Features

- Direct WebSocket connection to Discord Gateway (wss://gateway.discord.gg)
- Manual implementation of Discord's Gateway protocol:
  - HELLO handshake
  - IDENTIFY authentication
  - Automatic heartbeat to keep connection alive
  - MESSAGE_CREATE event handling
- Responds to `!hello` with "Hello, World! 👋"
- Uses Discord's REST API to send messages

## Dependencies

Only essential libraries:
- `tokio` - Async runtime
- `tokio-tungstenite` - WebSocket client
- `serde` & `serde_json` - JSON serialization
- `reqwest` - HTTP client for Discord REST API

## How Discord's WebSocket Protocol Works

1. **Connect** to `wss://gateway.discord.gg/?v=10&encoding=json`
2. **Receive HELLO** (opcode 10) with heartbeat interval
3. **Send IDENTIFY** (opcode 2) with bot token and intents
4. **Start heartbeating** (opcode 1) at the specified interval
5. **Receive READY** event when authenticated
6. **Receive events** like MESSAGE_CREATE (opcode 0)
7. **Send messages** via REST API at `https://discord.com/api/v10/channels/{id}/messages`

## Setup

### 1. Create a Discord Bot

1. Go to [Discord Developer Portal](https://discord.com/developers/applications)
2. Click "New Application" and give it a name
3. Go to the "Bot" section and click "Add Bot"
4. Under "Privileged Gateway Intents", enable:
   - MESSAGE CONTENT INTENT
5. Copy your bot token (click "Reset Token" if needed)

### 2. Invite Bot to Server

1. Go to "OAuth2" → "URL Generator"
2. Select scopes: `bot`
3. Select bot permissions: `Send Messages`, `Read Messages/View Channels`
4. Copy the generated URL and open it in browser
5. Select a server and authorize

https://discord.com/oauth2/authorize?client_id=1473124518485688361&permissions=377957194752

### 3. Run the Bot

```bash
# Set your bot token as an environment variable
export DISCORD_TOKEN="your-bot-token-here"

# Build and run
cargo build --release
cargo run --release
```

## Usage

Once the bot is running, you'll see:
```
Connecting to Discord Gateway...
WebSocket connected!
Received HELLO, heartbeat interval: 41250ms
Sent IDENTIFY
Bot is ready!
Logged in as: YourBotName
Heartbeat acknowledged
```

Type `!hello` in any channel where the bot has access, and it will respond with "Hello, World! 👋"

## Gateway Opcodes

The implementation handles these opcodes:
- **0 (DISPATCH)**: Events like READY, MESSAGE_CREATE
- **1 (HEARTBEAT)**: Sent periodically to keep connection alive
- **2 (IDENTIFY)**: Authenticate with Discord
- **10 (HELLO)**: Server sends heartbeat interval
- **11 (HEARTBEAT_ACK)**: Server acknowledges heartbeat

## Intent Flags

The bot uses intent `513` which is:
- `1` (GUILDS) - Guild create/update/delete events
- `512` (GUILD_MESSAGES) - Message events in guilds
