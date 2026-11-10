// The nofall's brain: pure standard library, no game types, so it compiles
// and tests wherever — the game's motion code is a world away from this
// crate.
//
// Meteor Client's NoFall ("Attempts to prevent you from taking fall
// damage.", category Movement) keeps its player whole four ways in
// multiplayer: spoofing the movement packets' on-ground flag (Packet),
// never telling the server it stands (NoGround), placing a block or a
// bucket mid-air beneath the player (AirPlace, Place) — answers to
// server-side anticheat, where only the server computes the damage and
// the client must lie or clutch its way out of it. Horizon's single
// player is different: the integrated server's own Game on the Server
// thread computes and applies the hurt, in this very process, and the
// game's own `native_living_fall` hook (its mod API for a living
// entity's fall) hands every mod a `cancel` flag at exactly that point.
// So this port is the module's promise without the multiplayer games: a
// toggle that cancels the local player's fall damage at the game's own
// hook, leaving the landing itself — physics, sounds of the blocks, the
// fall-distance bookkeeping — exactly as the game runs it. The original
// is GPL-3.0; this port is GPL-3.0 and credits it (see README.md).
/// The mod's registry id: the settings folder and the Options > Mods row.
pub const ID: &str = "nofall";
/// The version the row shows.
pub const VERSION: &str = "0.1.0";
/// The original's description, kept for the same promise.
pub const DESCRIPTION: &str = "Attempts to prevent you from taking fall damage.";
/// The toggle's setting file (one file per key under the mod's data dir),
/// which every tick re-reads from disk: the window's Games and the server's
/// own all see a change the moment it is written, on any thread.
pub const SETTING: &str = "enabled";

/// Whether the toggle is on: the setting file says `true`. A missing file
/// is off — the setting's default — and so is anything else it may hold.
pub fn setting_on(get: impl FnOnce(&str) -> Option<String>) -> bool {
    get(SETTING).is_some_and(|v| v.trim() == "true")
}

/// The settings page's single row.
pub fn label(on: bool) -> String {
    format!("Enabled: {}", if on { "ON" } else { "OFF" })
}

/// A click on the settings page's row `i`: the first row toggles the mod,
/// the rest are nothing (the page has the one row). Returns the new value
/// when the click was the toggle's, so the caller stores only real clicks.
pub fn click(i: usize, on: bool) -> Option<bool> {
    (i == 0).then_some(!on)
}

/// What to store for a value: the one setting key and its file text.
pub fn stored(on: bool) -> (&'static str, String) {
    (SETTING, if on { "true" } else { "false" }.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setting(value: &str) -> impl Fn(&str) -> Option<String> + '_ {
        move |key| (key == SETTING).then(|| value.to_owned())
    }

    /// A mod installed and never touched: the setting file does not exist
    /// yet, and the toggle is off — the default the mission asks for.
    #[test]
    fn a_missing_setting_file_is_off() {
        assert!(!setting_on(|_| None), "the default must be off");
        assert!(
            !setting_on(|key| (key == SETTING).then(String::new)),
            "an empty file is off"
        );
    }

    /// The file the click writes turns the toggle on, and any other text
    /// keeps it off (only `true` means on, trimmed like the game reads it).
    #[test]
    fn only_the_true_file_is_on() {
        assert!(setting_on(setting("true")));
        assert!(setting_on(setting(" true \n")));
        assert!(!setting_on(setting("false")));
        assert!(!setting_on(setting("TRUE")));
        assert!(!setting_on(setting("on")));
        // The stored text of either state reads back as that state.
        for on in [true, false] {
            let (key, value) = stored(on);
            assert_eq!(key, SETTING);
            assert!(
                setting_on(setting(&value)) == on,
                "{value} must read back {on}"
            );
        }
    }

    /// The settings page shows the one row in the state the files hold,
    /// and a click on it flips the state; clicks elsewhere change nothing.
    #[test]
    fn the_first_click_toggles_and_the_rest_are_nothing() {
        let (on, shown) = (false, label(false));
        assert_eq!(shown, "Enabled: OFF");
        assert_eq!(click(0, on), Some(true));
        assert_eq!(label(click(0, on).unwrap()), "Enabled: ON");
        assert_eq!(click(1, on), None, "the page has the one row");
        assert_eq!(click(7, true), None);
    }
}
