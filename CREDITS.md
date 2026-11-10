# Credits

- **Meteor Client** for Minecraft Java Edition — by the Meteor Development
  team, at <https://github.com/MeteorDevelopment/meteor-client>. Its NoFall
  module ("Attempts to prevent you from taking fall damage.", category
  Movement) is what this port is a port of; Meteor Client is
  GPL-3.0-licensed, and so is this port. The module keeps its player whole
  in multiplayer four ways (Packet, NoGround, AirPlace, Place — server
  anticheat answers and bucket clutches); this Horizon port takes the
  module's promise — no fall damage while enabled — to Horizon's own
  living-fall cancellation hook, single-player's honest path. None of the
  original's code is copied or translated; the description string is the
  original's promise, kept word for word, and the toggle's behavior is the
  module's own idea.
- **Horizon** — <https://github.com/horizon-rust/horizon>, whose
  `native_living_fall` (API 357, "a living entity's fall, with changeable
  distance and multiplier and cancellation") does the actual work, and
  whose movement tests (`a_clients_own_fall_does_not_hurt_it`) told this
  port where the hurt is applied.
- Vanilla Minecraft's fall damage naming, kept by Horizon's doc comments,
  guided the wording here.
