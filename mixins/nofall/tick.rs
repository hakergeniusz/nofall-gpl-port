// Once a tick, in every `Game`: the window's (in single player, on LAN and
// on servers) and the server's own. Only the window's writes the gate —
// the server's Games have no player of their own (a dedicated server
// simulates every player as another's; the single-player server thread
// swaps each turn's body in for its turn), and a server writing here
// would name bodies that are other people's to hurt.
//
// The window's own player is identified the way the game itself does it:
// by the world entity id the server assigned and the window carries in
// `Remote.entity_id` — the same id the authoritative sims on the Server
// thread receive in `native_living_fall`. The toggle is read live: the
// Options > Mods row from the window's own settings (this is the Game the
// row toggles in) and the Enabled file straight from disk, so a settings
// change lands within the tick that follows it, and the mod's state is
// the same whatever the thread reading the gate.
fn nofall_tick(game: &mut crate::game::Game<'_>) {
    use crate::game::nofall as nofall;
    if !game.native_window() {
        return;
    }
    let id = game.remote.as_ref().map(|r| r.entity_id as u32).unwrap_or(0);
    let on = id != 0
        && game.native_on(nofall::ID)
        && nofall::setting_on(|key| game.native_setting(nofall::ID, key));
    horizon_client::landing::nofall_gate::set_local_player(on, id);
}
