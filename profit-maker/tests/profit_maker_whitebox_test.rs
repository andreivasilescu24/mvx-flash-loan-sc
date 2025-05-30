use multiversx_sc_scenario::imports::*;
use profit_maker::*;

const OWNER: TestAddress = TestAddress::new("owner");
const ADDER_ADDRESS: TestSCAddress = TestSCAddress::new("adder");
const CODE_PATH: MxscPath = MxscPath::new("mxsc:output/profit-maker.mxsc.json");

fn world() -> ScenarioWorld {
    let mut blockchain = ScenarioWorld::new();

    // blockchain.set_current_dir_from_workspace("relative path to your workspace, if applicable");
    blockchain.register_contract(CODE_PATH, profit_maker::ContractBuilder);
    blockchain
}
