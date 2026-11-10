// The nofall's gate: which entity ids the mod protects, and who says so.
// Pure standard library, so it compiles wherever landing does — the game's
// windows are a world away from this crate.
//
// One process runs several Games: the window's (in single player the
// window is a client of its own integrated server through memory), the
// server thread's own, a splitscreen seat's beside them. The state is
// process-wide on purpose — this is the one flag whose value means the
// same thing on every thread ("the mod is enabled"), unlike the freecam's
// per-thread freeze, because both of its halves are the same for every
// Game that writes it:
//
// - `on` comes from the window's own settings (the Options > Mods row and
//   the toggle's setting file), and only the window's Game writes at all:
//   the server's Games have no player of their own (their `native_tick`
//   returns early), and the bodies they sim for their turns are guests
//   whose falls must never be canceled — a LAN guest's fall damage is
//   the host's to apply (and the guest's own game to want), and a
//   dedicated server's players are all "other players". Only the window
//   knows its local player, by the entity id the server assigned it and
//   its own client copy carries.
//
// - the ids are a set, not one value, because of splitscreen: each seat's
//   window writes its own player's id into the same set, and no seat's
//   write is another seat's to clear (each removes only its own id when
//   its mod turns off or its world closes). The readers on the Server
//   thread — the landing that applies the hurt — see the union: every
//   seat whose window said "protect mine".
//
// The entity id is what `native_living_fall` already receives, and what
// the authoritative sims carry: the hub gives each turn's body the
// player's world entity id (`lan_actions.rs`), the same id the window
// learned at login (`Remote.entity_id`). So the gate needs no Game
// access from the landing side — a bare id match decides.
use std::collections::BTreeSet;
use std::sync::{LazyLock, Mutex};

/// The entity ids whose falls the mod cancels, as the windows named them.
/// Empty — the field default — protects nobody: a landing that runs before
/// the first tick wrote the gate (a world just opened) takes no liberties.
static LOCAL: LazyLock<Mutex<BTreeSet<u32>>> = LazyLock::new(|| Mutex::new(BTreeSet::new()));

/// The window's tick says whether its player's falls are the mod's to
/// cancel. `id` is the player's world entity id; without one (the title
/// screen, a world just closed) or with the mod off, the id leaves the
/// set — a player's own game removing only its own id, never another
/// seat's. Id 0 is no entity: a body the game has not yet given an id to
/// (every fresh sim starts there) protects nothing and matches nothing.
pub fn set_local_player(on: bool, id: u32) {
    let mut local = LOCAL.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if on && id != 0 {
        local.insert(id);
    } else {
        local.remove(&id);
    }
}

/// Whether this fall — the `entity` id `native_living_fall` was handed —
/// is the mod's to cancel. The kind filter lives with the caller: mobs and
/// other players' kinds never reach the set's ids anyway, but the guard
/// here keeps an id of 0 (an unnamed body) from ever matching.
pub fn protects(entity: u32) -> bool {
    entity != 0 && LOCAL.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).contains(&entity)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The state one window writes is the state every thread reads, and
    /// each writer's removal takes only its own id — the seat that leaves
    /// never clears a seat that stays. Id 0 protects nobody, whatever the
    /// writes around it say.
    #[test]
    fn nofall_gate_holds_each_window_its_own_ids() {
        set_local_player(true, 42);
        assert!(protects(42), "the window's player is protected");
        assert!(!protects(0), "an unnamed body matches nothing");
        assert!(!protects(41) && !protects(43), "nobody else is protected by it");
        // Another seat writes its own id: both stand, and the union is
        // what the Server thread's landing reads.
        set_local_player(true, 43);
        assert!(protects(42) && protects(43));
        // The second seat's game turns off or closes: the first seat's
        // protection is not the removal's to take.
        set_local_player(false, 43);
        assert!(protects(42));
        assert!(!protects(43));
        // A write with no id (no player) removes nothing that exists.
        set_local_player(true, 0);
        assert!(protects(42), "id 0 is no id: the set is untouched");
        set_local_player(false, 0);
        assert!(protects(42));
        // And the last write out leaves the set empty for the next test.
        set_local_player(false, 42);
        assert!(!protects(42));
        // The set is process state: another thread reads what this one
        // wrote, as the Server thread's landing reads the window's gate.
        set_local_player(true, 7);
        std::thread::scope(|s| {
            s.spawn(|| assert!(protects(7), "the gate is every thread's to read"))
                .join()
                .unwrap();
        });
        set_local_player(false, 7);
    }

    /// The reviewer's case, through the game's own motion code: the
    /// protected player lands from a 30-block fall and takes no hurt —
    /// while the landing itself stays whole (the ground holds, the fall
    /// distance resets) — and a second player, whose id the gate never
    /// held, is hurt by the very same fall in the very same gate state,
    /// as a LAN guest's and a dedicated server's player must be. With the
    /// gate cleared, the protected id hurts again, so the harness's own
    /// hurt is what the mod's absence leaves.
    #[test]
    fn nofall_cancels_its_player_fall_and_nobody_elses() {
        // The prepared resources carry the movement properties the sim
        // loads; the lan bot tests skip without them, and so does this.
        let root = std::env::var_os("HORIZON_TEST_RESOURCES")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| {
                std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                    .join("../../.local/horizon-client")
                    .join(horizon_core::VANILLA_VERSION)
            });
        if !root.join("movement-properties.nbt").exists() {
            eprintln!("skipped the nofall landing test: prepared resources unavailable");
            return;
        }
        let assets = crate::assets::Assets::load(&root).unwrap();
        let resources = crate::motion::MotionResources::load(&root, &assets).unwrap();
        let binding = crate::collisions::load(&root, assets.registry).unwrap();
        // A stone floor a few blocks down, like the lan bot tests' world.
        const STONE: u32 = 1;
        let floor = |scene: &mut crate::world::World| {
            for cz in -1..=1 {
                for cx in -1..=1 {
                    let mut chunk = crate::chunk::Chunk::empty(cx, cz);
                    for z in 0..16 {
                        for x in 0..16 {
                            chunk.set(x, crate::chunk::MIN_Y + 4, z, STONE);
                        }
                    }
                    chunk.lit = true;
                    scene.chunks.insert((cx, cz), std::sync::Arc::new(chunk));
                }
            }
        };
        let scene_of = || {
            let mut scene = crate::world::World::new(horizon_world::registry::BlockStateId(0), 0);
            floor(&mut scene);
            scene
        };
        // Ten blocks above the floor, a 30-block fall distance like the
        // game's own landing test: the drop lands, and the hurt is what
        // the distance asks for (`move_self` is `LocalPlayer.move`).
        let air = horizon_core::geometry::Vec3::new(0.5, f64::from(crate::chunk::MIN_Y + 14), 0.5);
        let fall = |player: &mut crate::motion::PlayerMotion,
                    scene: &mut crate::world::World,
                    assets: &crate::assets::Assets,
                    binding: &_,
                    resources: &_| {
            player.fall_distance = 30.0;
            crate::motion::MotionWorld {
                entities: &[],
                player,
                scene,
                assets,
                collision_binding: binding,
                resources,
                health: &resources.health,
            }
            .move_self(horizon_core::geometry::Vec3::new(0.0, -30.0, 0.0))
            .unwrap();
        };

        // The gate holds the window's player (entity id 42): their fall
        // lands without the hurt — health whole, distance reset by the
        // landing the mod never touched.
        set_local_player(true, 42);
        let mut scene = scene_of();
        let mut owner = crate::motion::PlayerMotion::new(&resources, air).unwrap();
        owner.entity_id = 42;
        fall(&mut owner, &mut scene, &assets, &binding, &resources);
        assert!(
            owner.ground.on_ground,
            "the canceled fall still lands: the mod touches no physics"
        );
        assert_eq!(
            owner.fall_distance, 0.0,
            "the landing resets the fall distance as always"
        );
        assert_eq!(
            owner.health.health(),
            20.0,
            "the protected player's fall hurts no more"
        );

        // A second player in the same gate state — a LAN guest's id, a
        // dedicated server's player: the gate names 42 alone, so this
        // body takes the hurt in full.
        let mut guest = crate::motion::PlayerMotion::new(&resources, air).unwrap();
        guest.entity_id = 44;
        fall(&mut guest, &mut scene, &assets, &binding, &resources);
        assert!(
            guest.health.health() < 20.0,
            "nobody else's fall is the mod's to cancel"
        );
        drop(scene);

        // The gate cleared (the mod turned off, the world closed): the
        // same id takes the hurt again — the harness's own landing is
        // what the mod's absence leaves.
        set_local_player(false, 42);
        let mut scene = scene_of();
        let mut owner_again = crate::motion::PlayerMotion::new(&resources, air).unwrap();
        owner_again.entity_id = 42;
        fall(&mut owner_again, &mut scene, &assets, &binding, &resources);
        assert!(
            owner_again.health.health() < 20.0,
            "with the mod off the fall hurts, as the game runs it"
        );
    }
}
