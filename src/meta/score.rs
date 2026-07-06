// Scoreboard score formula (Phase 8, D2) — progress + speed.
//
// `progress` rewards how far a run got (act/node depth, character level) with a flat victory
// bonus; `speed` rewards finishing under a par time (never a penalty — a run that blows past the
// par time just earns zero speed bonus, on top of whatever progress it made). Pure function, no
// ECS/RNG, so every case (act/node/level/victory/time) is directly unit-testable; the constants
// are grouped here so tuning them never touches a call site.
//
// Called from run/systems/persistence.rs::record_run_end (both the defeat and victory paths).

/// A run finishing at or after this many seconds earns no speed bonus (only the progress term).
pub const TIME_PAR_SECS: f32 = 600.0; // 10 minutes
/// Seconds-under-par → score-points conversion.
pub const SPEED_WEIGHT: f32 = 2.0;
/// Flat bonus for actually clearing the Act-3 boss (vs. dying at the same depth/level).
pub const VICTORY_BONUS: f32 = 5000.0;

pub struct ScoreInput {
    pub act: u8,
    pub node_column: usize,
    pub level: u32,
    pub victory: bool,
    pub elapsed_secs: f32,
}

/// `progress = act*1000 + node_column*50 + level*100 + (victory ? 5000 : 0)`
/// `speed    = max(0, TIME_PAR_SECS - elapsed_secs) * SPEED_WEIGHT`
/// `score    = round(progress + speed)`
pub fn compute_score(input: &ScoreInput) -> u32 {
    let progress = input.act as f32 * 1000.0
        + input.node_column as f32 * 50.0
        + input.level as f32 * 100.0
        + if input.victory { VICTORY_BONUS } else { 0.0 };
    let speed = (TIME_PAR_SECS - input.elapsed_secs).max(0.0) * SPEED_WEIGHT;
    (progress + speed).round() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(act: u8, node_column: usize, level: u32, victory: bool, elapsed_secs: f32) -> ScoreInput {
        ScoreInput { act, node_column, level, victory, elapsed_secs }
    }

    #[test]
    fn deeper_act_scores_higher_than_a_shallower_one_at_the_same_level() {
        let shallow = compute_score(&input(1, 0, 5, false, 300.0));
        let deep = compute_score(&input(2, 0, 5, false, 300.0));
        assert!(deep > shallow, "reaching act 2 should outscore stalling in act 1");
    }

    #[test]
    fn further_node_column_scores_higher_at_the_same_act_and_level() {
        let early = compute_score(&input(1, 2, 5, false, 300.0));
        let later = compute_score(&input(1, 10, 5, false, 300.0));
        assert!(later > early);
    }

    #[test]
    fn higher_level_scores_higher_at_the_same_depth() {
        let low = compute_score(&input(1, 5, 3, false, 300.0));
        let high = compute_score(&input(1, 5, 10, false, 300.0));
        assert!(high > low);
    }

    #[test]
    fn victory_adds_a_flat_bonus_over_an_identical_defeat() {
        let defeat = compute_score(&input(3, 14, 20, false, 300.0));
        let victory = compute_score(&input(3, 14, 20, true, 300.0));
        assert_eq!(victory - defeat, VICTORY_BONUS as u32);
    }

    #[test]
    fn faster_clears_score_higher_than_slower_ones() {
        let fast = compute_score(&input(2, 3, 8, false, 60.0));
        let slow = compute_score(&input(2, 3, 8, false, 400.0));
        assert!(fast > slow, "finishing sooner should score higher, all else equal");
    }

    #[test]
    fn time_at_or_beyond_par_contributes_no_speed_bonus_and_never_goes_negative() {
        let at_par = compute_score(&input(1, 0, 1, false, TIME_PAR_SECS));
        let way_over = compute_score(&input(1, 0, 1, false, TIME_PAR_SECS * 10.0));
        assert_eq!(at_par, way_over, "no bonus at/after par — but also no penalty for taking longer");
    }
}
