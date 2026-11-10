# NoFall for Horizon

Attempts to prevent you from taking fall damage. While **Enabled** is on,
your falls land exactly as the game runs them — the ground holds, the fall
distance resets, the landing sounds of blocks play — and only the damage is
gone. Off by default; the switch is in **Options > Mods > NoFall**.

A Horizon native mod (a mixin bundle the launcher compiles on the pinned
Horizon revision) ported from the **NoFall** module of
[Meteor Client](https://github.com/MeteorDevelopment/meteor-client)
("Attempts to prevent you from taking fall damage.", category Movement).
Meteor Client is GPL-3.0; this port is GPL-3.0 too and credits it — see
[LICENSE](LICENSE) and [CREDITS.md](CREDITS.md).

## How it works

- Once a tick, the window's game tells the mod's gate which entity id is
  its local player (the id the server assigned, carried the same way the
  game's own code identifies the player), while **Enabled** says so.
- The hurt of a fall is applied by the integrated server's own game on the
  Server thread, at the game's own hook for a living entity's fall
  (`native_living_fall`, the point where Horizon itself hands mods a
  cancellation). There the mod cancels the fall of a player whose id the
  gate holds — and nobody else's: mobs land untouched, LAN guests take
  their damage as the host applies it, and a dedicated server's players
  are never protected (the server has no window, so it never writes the
  gate).
- A canceled fall still lands: physics, the block landing rules, the fall
  statistic and the fall distance bookkeeping all run. The one thing the
  cancel also takes is the damage fall sound (the big thud), which the
  game plays with the damage it no longer deals.

**Multiplayer against a remote server is out of reach for any client mod**:
there the damage is computed on that server, which no client can cancel.
The same is true of the original's clever ways around it — its four modes
(Packet: spoofing the on-ground flag of movement packets; NoGround: never
claiming ground; AirPlace/Place: clutching with a block or a bucket) are
answers to server-side anticheat, where only the server computes the damage
and the client must lie or clutch its way out of it. They are out of scope
for this port's v1: Horizon's single player hands the mod the game's own
cancellation point, so the honest toggle is the whole module's promise
here. LAN guests against a host are likewise server-side for their game.

## Install

Put `nofall-<version>.hnmod` in your launcher's mods (or install it from
the registry) and rebuild the native client. Settings: **Options > Mods >
NoFall > Enabled** — one toggle, off by default.

## License

GPL-3.0-only, like the original module this ports. This is a
reimplementation of the module's observable behavior against Horizon's own
hooks, written from scratch in Rust: none of Meteor Client's code is
copied or translated, and the description string is the original's own
promise, kept word for word. See [LICENSE](LICENSE) and
[CREDITS.md](CREDITS.md).
