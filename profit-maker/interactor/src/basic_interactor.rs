mod basic_interactor_cli;
mod basic_interactor_config;
mod basic_interactor_state;

pub use basic_interactor_config::Config;
use basic_interactor_state::State;
use clap::Parser;
use profit_maker::profit_maker_proxy;

use multiversx_sc_snippets::imports::*;

const ADDER_CODE_PATH: MxscPath = MxscPath::new("../output/adder.mxsc.json");

pub async fn adder_cli() {
    env_logger::init();

    let config = Config::load_config();

    let mut basic_interact = ProfitMakerInteract::new(config).await;

    let cli = basic_interactor_cli::InteractCli::parse();
    match &cli.command {
        Some(basic_interactor_cli::InteractCliCommand::Deploy) => {
            basic_interact.deploy().await;
        }
        Some(basic_interactor_cli::InteractCliCommand::Upgrade(args)) => {
            let owner_address = basic_interact.adder_owner_address.clone();
            basic_interact
                .upgrade(args.value, &owner_address, None)
                .await
        }
        Some(basic_interactor_cli::InteractCliCommand::Add(args)) => {
            basic_interact.add(args.value).await;
        }
        Some(basic_interactor_cli::InteractCliCommand::Sum) => {
            let sum = basic_interact.get_sum().await;
            println!("sum: {sum}");
        }
        None => {}
    }
}

pub struct ProfitMakerInteract {
    pub interactor: Interactor,
    pub adder_owner_address: Bech32Address,
    pub wallet_address: Bech32Address,
    pub state: State,
}

impl ProfitMakerInteract {
    pub async fn new(config: Config) -> Self {
        let mut interactor = Interactor::new(config.gateway_uri())
            .await
            .use_chain_simulator(config.use_chain_simulator());
        interactor.set_current_dir_from_workspace("contracts/examples/adder/interactor");

        let adder_owner_address = interactor.register_wallet(test_wallets::heidi()).await;
        let wallet_address = interactor.register_wallet(test_wallets::ivan()).await;

        interactor.generate_blocks(30u64).await.unwrap();

        ProfitMakerInteract {
            interactor,
            adder_owner_address: adder_owner_address.into(),
            wallet_address: wallet_address.into(),
            state: State::load_state(),
        }
    }

    pub async fn deploy(&mut self) {
        let new_address = self
            .interactor
            .tx()
            .from(&self.adder_owner_address.clone())
            .gas(100_000_000)
            .typed(profit_maker_proxy::ProfitMakerProxy)
            .init(0u64)
            .code(ADDER_CODE_PATH)
            .returns(ReturnsNewBech32Address)
            .run()
            .await;

        println!("new address: {new_address}");
        self.state.set_adder_address(new_address);
    }
}
