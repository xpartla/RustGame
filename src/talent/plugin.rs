// TalentPlugin — wires the talent system into the app (Phase 2).
//
// Responsibilities:
//   - Registers TalentDef as a Bevy asset + its RON loader (*.talent.ron).
//   - Registers TalentLibrary (id → handle) and loads the talent RON files at startup.
//   - Attaches AcquiredTalents / ActiveHooks to the player on spawn.
//   - Registers TalentAcquiredEvent / TalentRemovedEvent and the install/uninstall systems.
//
// The install/uninstall systems run ungated by GameState: TalentAcquiredEvent is emitted from
// the TalentPicker state, so its reader must not be frozen with the InRun world.

use bevy::prelude::*;
use crate::core::def_library::DefLibraryAppExt;
use crate::core::sets::CombatSet;
use crate::core::systems::apply_heal::apply_heal;
use crate::game::state::GameState;
use crate::talent::assets::TalentDef;
use crate::talent::systems::apply::{
    attach_talent_components, install_acquired_talent, uninstall_removed_talent,
    TalentAcquiredEvent, TalentRemovedEvent,
};
use crate::talent::systems::merchant::{
    handle_merchant_input, handle_merchant_remove, handle_merchant_trade, MerchantRemoveRequest,
    MerchantTradeRequest,
};
use crate::talent::systems::passives::{enforce_heal_cap, overkill_leech_on_kill, resolve_health_and_healing};
use crate::ability::systems::summon::update_minion_lifecycle;
use crate::player::systems::base_stats::apply_base_stats;

pub struct TalentPlugin;

impl Plugin for TalentPlugin {
    fn build(&self, app: &mut App) {
        // TalentDef asset + RON loader + TalentLibrary + Startup populate, in one call.
        app.register_def_library::<TalentDef>()
            .add_event::<TalentAcquiredEvent>()
            .add_event::<TalentRemovedEvent>()
            .add_event::<MerchantRemoveRequest>()
            .add_event::<MerchantTradeRequest>();

        // attach runs in Update (not Startup) so `Added<Player>` reliably fires after the
        // Startup `spawn_player` — Startup system ordering relative to it is otherwise undefined.
        app.add_systems(
            Update,
            (attach_talent_components, install_acquired_talent, uninstall_removed_talent),
        );

        // Merchant ops (Phase 7.5E). The ops handlers run only on a frame carrying their request; the
        // overlay input runs only in the Merchant state — both inert in the runless campaign.
        app.add_systems(
            Update,
            (
                handle_merchant_remove.run_if(on_event::<MerchantRemoveRequest>),
                handle_merchant_trade.run_if(on_event::<MerchantTradeRequest>),
            ),
        );
        app.add_systems(
            Update,
            handle_merchant_input.run_if(in_state(GameState::Merchant)),
        );

        // Class-passive consumers (Phase 9.2) that don't fit the per-cast hook pipeline — see
        // talent/systems/passives.rs's module doc comment.
        //
        // All three are pinned `.after(install_acquired_talent).after(uninstall_removed_talent)
        // .after(apply_base_stats)`: found via Bevy's ambiguity checker
        // (`ScheduleBuildSettings { ambiguity_detection: LogLevel::Error }`, used to hunt a golden-
        // campaign reproducibility flake this whole set of pins closes). Those three systems run
        // ungated/every-frame and can mutate `AcquiredTalents`/`ActiveHooks`/`Health` the SAME frame
        // these read them; with no explicit order, whether a same-frame talent acquisition (or the
        // base_stats correction) is visible THIS frame or only the NEXT was free to vary between
        // separate schedule builds — exactly the kind of one-frame timing gap that compounds into a
        // real trace divergence over a long campaign. Pinning makes "acquired/removed/corrected this
        // frame ⇒ visible to these consumers this same frame" a deterministic guarantee instead of
        // an accident of scheduler tie-breaking.
        app.add_systems(
            Update,
            (
                enforce_heal_cap
                    .in_set(CombatSet::Apply)
                    .after(apply_heal)
                    .after(crate::core::systems::apply_damage::apply_damage)
                    .after(crate::ability::systems::purgatory::purgatory_cheat_death)
                    .after(install_acquired_talent)
                    .after(uninstall_removed_talent)
                    .after(apply_base_stats),
                overkill_leech_on_kill
                    .in_set(CombatSet::Death)
                    .after(install_acquired_talent)
                    .after(uninstall_removed_talent)
                    .after(apply_base_stats),
                // Not damage/death-ordering-sensitive — anchored `.after(CombatSet::Death)` anyway
                // (mirrors `gain_experience`/the ability grant chain) rather than left fully
                // unordered, per the Phase 9.2 scheduling lesson (ability/plugin.rs's doc comment
                // on the Companion grant-chain race): adding an unordered system can silently shift
                // the scheduler's tie-break order for OTHER unordered systems elsewhere.
                resolve_health_and_healing
                    .after(CombatSet::Death)
                    .after(update_minion_lifecycle)
                    .after(install_acquired_talent)
                    .after(uninstall_removed_talent)
                    .after(apply_base_stats),
            )
                .run_if(in_state(GameState::InRun)),
        );
    }
}
