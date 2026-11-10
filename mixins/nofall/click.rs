// A click on the mod's settings page: flip Enabled and keep it in the
// setting file the window's tick re-reads, so the gate follows within a
// tick of the click. The stored file (not the gate) is the source of
// truth: it is what survives a restart, what a player may edit by hand,
// and what the original's config kept.
fn nofall_click(game: &mut crate::game::Game<'_>, mod_id: &str, button: usize) {
    use crate::game::nofall as nofall;
    if mod_id != nofall::ID {
        return;
    }
    let clicked = {
        let game = &*game;
        nofall::click(button, nofall::setting_on(|key| game.native_setting(nofall::ID, key)))
    };
    if let Some(on) = clicked {
        let (key, value) = nofall::stored(on);
        game.native_set_setting(nofall::ID, key, &value);
    }
}
