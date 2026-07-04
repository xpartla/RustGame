// RunRng — the seeded, deterministic random number generator for a run.
//
// RULE: any system whose output must be reproducible from the run seed takes
// ResMut<RunRng>. Everything else (VFX particle angles, audio variation, etc.)
// uses rand::thread_rng() directly. Mixing the two is the bug this separation prevents.
//
// RunRng is part of RunState's serialized blob. On "Resume Run", the RNG state is
// restored from the save so offers, map generation, and pack composition pick up
// mid-stream rather than restarting from the seed. This keeps a resumed run identical
// to one that was never interrupted.
//
// Seeded from RunState.seed via SmallRng::seed_from_u64.

use bevy::prelude::*;
use rand::prelude::*;

/// Resource. One instance per active run. Absent between runs.
#[derive(Resource)]
pub struct RunRng(pub SmallRng);

impl RunRng {
    pub fn from_seed(seed: u64) -> Self {
        Self(SmallRng::seed_from_u64(seed))
    }

    pub fn rng(&mut self) -> &mut SmallRng {
        &mut self.0
    }
}
