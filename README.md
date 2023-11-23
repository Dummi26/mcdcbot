# mcdcbot

- shut down / start your minecraft server from discord
- switch between multiple different worlds (/servers) from discord
- forward messages from a certain discord channel to the minecraft chat, and minecraft chat messages to that discord channel

## usage

See the repository for examples, notable `mcdcbot/servers/*` and `mcdcbot/settings.txt`.

For advanced config options, check `minecraft_manager/src/lib.rs`, especially the `fn from_lines()`.

Documentation may be added in the future...

### In Discord:

`/list` lists Servers:

- (m) My Server
- (t) Test World

`/starts` starts a server. You can choose which one to start.
Only one server can be running at any given time.

`/start m` or `/start My Server`

`/start t` or `/start Test World`

`/stop` runs the `stop` command in the current server.
Once the server shuts down, a message will be sent.
