use multiversx_sc_scenario::imports::*;

use profit_maker::*;

const OWNER_ADDRESS: TestAddress = TestAddress::new("owner");
const ADDER_ADDRESS: TestSCAddress = TestSCAddress::new("adder");
const CODE_PATH: MxscPath = MxscPath::new("output/profit-maker.mxsc.json");

fn world() -> ScenarioWorld {
    let mut blockchain = ScenarioWorld::new();

    // blockchain.set_current_dir_from_workspace("relative path to your workspace, if applicable");
    blockchain.register_contract(CODE_PATH, profit_maker::ContractBuilder);
    blockchain
}

#[test]
fn adder_blackbox() {
    let mut world = world();

    world.start_trace();

    world.account(OWNER_ADDRESS).nonce(1);

    let new_address = world
        .tx()
        .from(OWNER_ADDRESS)
        .typed(profit_maker_proxy::ProfitMakerProxy)
        .init(5u32)
        .code(CODE_PATH)
        .new_address(ADDER_ADDRESS)
        .returns(ReturnsNewAddress)
        .run();

    assert_eq!(new_address, ADDER_ADDRESS.to_address());

    world.check_account(OWNER_ADDRESS);

    world
        .check_account(ADDER_ADDRESS)
        .check_storage("str:sum", "6");

    world
        .tx()
        .from(OWNER_ADDRESS)
        .to(ADDER_ADDRESS)
        .typed(profit_maker_proxy::ProfitMakerProxy)
        .upgrade()
        .code(CODE_PATH)
        .run();

    world
        .check_account(ADDER_ADDRESS)
        .check_storage("str:sum", "100");

    world.write_scenario_trace("trace1.scen.json");
}
