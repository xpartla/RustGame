// Golden scenario — class-resource charges (Phase 9.1).
//
// No shipped hero uses `ResourceModel::Charges` yet (Mage frost charges / Druid combo charges land
// in Phase 9.4/9.5). This proves the HUD bridge: whatever grants/spends `Charges` gets a working
// `ClassResource` bar for free. `Charges`'s own gain/spend_all math is unit-tested directly in
// hero/components.rs.

use rust_game::sim::Sim;

#[test]
fn charges_sync_into_the_hud_class_resource_bar() {
    let mut sim = Sim::new_arena(42);
    let player = sim.player();
    assert_eq!(sim.class_resource(player), None, "no ClassResource until Charges appears");

    sim.set_charges(player, 2, 5);
    sim.step(1);
    assert_eq!(sim.class_resource(player), Some((2.0, 5.0)), "bridge mirrors current/max");

    sim.set_charges(player, 5, 5);
    sim.step(1);
    assert_eq!(sim.class_resource(player), Some((5.0, 5.0)), "bridge tracks further changes");
}
