// A living entity's fall, as the game asks its mods (`native_living_fall`:
// API 357, "with changeable distance and multiplier and cancellation").
// This is the game's own cancellation point for exactly the hurt this mod
// exists to stop — a `cancel` here keeps the whole landing as the game
// runs it (the ground holds, the fall distance resets, the block landing
// rules and the statistic run) and drops only the damage and the damage
// fall sound it carries.
//
// The gate names the entity ids the windows protect, and the id is what
// the authoritative sims carry: the hurt is applied on the Server thread
// (a client-side sim's hurt is a no-op by itself), where the landing runs
// with the player's world entity id — the same id the window learned at
// login. Mobs land through this hook too, under their own kinds, and the
// kind filter sends them through untouched; so do LAN guests and every
// dedicated server's player, whose ids no window ever wrote.
fn nofall_living_fall(entity: u32, kind: &str, cancel: &mut bool) {
    if kind == "minecraft:player" && crate::landing::nofall_gate::protects(entity) {
        *cancel = true;
    }
}
