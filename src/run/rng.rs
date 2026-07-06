// RunRng — the seeded, deterministic random number generator for a run.
//
// RULE: any system whose output must be reproducible from the run seed takes
// ResMut<RunRng>. Everything else (VFX particle angles, audio variation, etc.)
// uses rand::thread_rng() directly. Mixing the two is the bug this separation prevents.
//
// RunRng is part of the SavedRun blob (meta/state.rs). On "Resume Run", the RNG state is
// restored from the save so offers, map generation, and pack composition pick up mid-stream
// rather than restarting from the seed — this keeps a resumed run identical to one that was
// never interrupted (Phase 8, D1).
//
// Algorithm (Phase 8, D1): ChaCha8Rng, not SmallRng. Two reasons, one forced the other:
//   - SmallRng has no serde support at all — resuming needs the exact stream position saved.
//   - SmallRng is *explicitly* not guaranteed stable across rand versions or platforms, so
//     even a seed+replay-N-draws scheme would be fragile; ChaCha8Rng's output is a documented,
//     versioned, portable stream — a bonus determinism win beyond just "can be serialized"
//     (see docs/testing.md's golden-baseline portability note).
//   - Switching algorithms changes the entire draw sequence for the same seed, so this move
//     required a one-time, declared golden-master regeneration (CHANGELOG "Phase 8").
//
// Seeded from RunState.seed via ChaCha8Rng::seed_from_u64.
//
// Serde note: `rand_chacha`'s own `serde1` feature would derive Serialize/Deserialize for us, but
// its wire format includes a `word_pos: u128` — and `ron` 0.8 (this project's save format) cannot
// serialize u128/i128 at all ("u128 is not supported"). So RunRng implements serde by hand below,
// using ChaCha8Rng's public `get_seed`/`get_stream`/`get_word_pos`/`set_stream`/`set_word_pos`
// accessors (ungated by any feature) and splitting the 128-bit word position into two u64 halves —
// otherwise identical in spirit to rand_chacha's own (seed, stream, word_pos) snapshot. No
// `rand_chacha` feature flags are needed for this.

use bevy::prelude::*;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Resource. One instance per active run. Absent between runs.
#[derive(Resource, Clone, Debug)]
pub struct RunRng(pub ChaCha8Rng);

impl RunRng {
    pub fn from_seed(seed: u64) -> Self {
        Self(ChaCha8Rng::seed_from_u64(seed))
    }

    pub fn rng(&mut self) -> &mut ChaCha8Rng {
        &mut self.0
    }
}

/// The RON-compatible wire format: identical fields to rand_chacha's own serde shape, except
/// `word_pos` (a 68-bit counter surfaced as `u128`) is split into two `u64` halves.
#[derive(Serialize, Deserialize)]
struct RunRngSnapshot {
    seed: [u8; 32],
    stream: u64,
    word_pos_hi: u64,
    word_pos_lo: u64,
}

impl Serialize for RunRng {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let word_pos = self.0.get_word_pos();
        RunRngSnapshot {
            seed: self.0.get_seed(),
            stream: self.0.get_stream(),
            word_pos_hi: (word_pos >> 64) as u64,
            word_pos_lo: word_pos as u64,
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for RunRng {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let snap = RunRngSnapshot::deserialize(deserializer)?;
        let mut rng = ChaCha8Rng::from_seed(snap.seed);
        // Order matters: `set_stream` may itself re-derive the word position if the buffer isn't
        // block-aligned, so it must run before the explicit `set_word_pos` (mirrors rand_chacha's
        // own Deserialize impl).
        rng.set_stream(snap.stream);
        let word_pos = ((snap.word_pos_hi as u128) << 64) | (snap.word_pos_lo as u128);
        rng.set_word_pos(word_pos);
        Ok(RunRng(rng))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::Rng;

    #[test]
    fn same_seed_same_stream() {
        let mut a = RunRng::from_seed(42);
        let mut b = RunRng::from_seed(42);
        let draws_a: Vec<u32> = (0..10).map(|_| a.rng().r#gen()).collect();
        let draws_b: Vec<u32> = (0..10).map(|_| b.rng().r#gen()).collect();
        assert_eq!(draws_a, draws_b, "ChaCha8Rng is deterministic: same seed ⇒ same sequence");
    }

    #[test]
    fn different_seed_different_stream() {
        let mut a = RunRng::from_seed(1);
        let mut b = RunRng::from_seed(2);
        let draws_a: Vec<u32> = (0..10).map(|_| a.rng().r#gen()).collect();
        let draws_b: Vec<u32> = (0..10).map(|_| b.rng().r#gen()).collect();
        assert_ne!(draws_a, draws_b);
    }

    /// The D1 contract at the type level: serializing mid-stream and restoring from that
    /// snapshot continues the exact same sequence — the whole reason for switching to ChaCha8.
    #[test]
    fn serialized_snapshot_restores_the_exact_stream_position() {
        let mut rng = RunRng::from_seed(0xC0FFEE);
        let _warmup: Vec<u32> = (0..17).map(|_| rng.rng().r#gen()).collect();

        let ron = ron::ser::to_string(&rng).expect("serialize mid-stream RunRng");
        let expected_next: Vec<u32> = (0..25).map(|_| rng.rng().r#gen()).collect();

        let mut restored: RunRng = ron::de::from_str(&ron).expect("deserialize RunRng snapshot");
        let actual_next: Vec<u32> = (0..25).map(|_| restored.rng().r#gen()).collect();

        assert_eq!(actual_next, expected_next, "resume must continue the exact draw sequence");
    }
}
