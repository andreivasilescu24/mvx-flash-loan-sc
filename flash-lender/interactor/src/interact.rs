#![allow(non_snake_case)]

pub mod config;
mod proxy;

use config::Config;
use multiversx_sc_snippets::imports::*;
use serde::{Deserialize, Serialize};
use std::{
    io::{Read, Write},
    path::Path,
};

const STATE_FILE: &str = "state.toml";

pub enum PayerWallet {
    Alice,
    Bob,
    MyWallet,
}
pub async fn flash_loan_cli() {
    env_logger::init();

    let mut args = std::env::args();
    let _ = args.next();
    let cmd = args.next().expect("at least one argument required");
    let config = Config::new();
    let mut interact = ContractInteract::new(config).await;
    match cmd.as_str() {
        // "deploy" => interact.deploy().await,
        "upgrade" => interact.upgrade().await,
        // "flashLoan" => interact.flash_loan().await,
        "flashLoanConfig" => interact.flash_loan_config().await,
        // "getMaxLoan" => interact.get_max_loan().await,
        "getMinLoan" => interact.min_loan_amount().await,
        "getFeeBasisPoints" => interact.fee_basis_points().await,
        _ => panic!("unknown command: {}", &cmd),
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct State {
    contract_address: Option<Bech32Address>,
}

impl State {
    // Deserializes state from file
    pub fn load_state() -> Self {
        if Path::new(STATE_FILE).exists() {
            let mut file = std::fs::File::open(STATE_FILE).unwrap();
            let mut content = String::new();
            file.read_to_string(&mut content).unwrap();
            toml::from_str(&content).unwrap()
        } else {
            Self::default()
        }
    }

    /// Sets the contract address
    pub fn set_address(&mut self, address: Bech32Address) {
        self.contract_address = Some(address);
    }

    /// Returns the contract address
    pub fn current_address(&self) -> &Bech32Address {
        self.contract_address
            .as_ref()
            .expect("no known contract, deploy first")
    }
}

impl Drop for State {
    // Serializes state to file
    fn drop(&mut self) {
        let mut file = std::fs::File::create(STATE_FILE).unwrap();
        file.write_all(toml::to_string(self).unwrap().as_bytes())
            .unwrap();
    }
}

pub struct ContractInteract {
    interactor: Interactor,
    alice_wallet_address: Address,
    bob_wallet_address: Address,
    my_wallet_address: Address,
    contract_code: BytesValue,
    state: State,
}

impl ContractInteract {
    pub async fn new(config: Config) -> Self {
        let mut interactor = Interactor::new(config.gateway_uri())
            .await
            .use_chain_simulator(config.use_chain_simulator());

        interactor.set_current_dir_from_workspace("flash-loan");
        let my_wallet = Wallet::from_pem_file("../../wallet.pem").unwrap();
        let alice_wallet = interactor.register_wallet(test_wallets::alice()).await;
        let bob_wallet = interactor.register_wallet(test_wallets::bob()).await;

        let my_wallet_address = interactor.register_wallet(my_wallet).await;

        // Useful in the chain simulator setting
        // generate blocks until ESDTSystemSCAddress is enabled
        interactor.generate_blocks_until_epoch(1).await.unwrap();

        let contract_code = BytesValue::interpret_from(
            "mxsc:../output/flash-loan.mxsc.json",
            &InterpreterContext::default(),
        );

        ContractInteract {
            interactor,
            alice_wallet_address: alice_wallet,
            bob_wallet_address: bob_wallet,
            my_wallet_address,
            contract_code,
            state: State::load_state(),
        }
    }

    pub async fn deploy(&mut self, basis_points: u32) {
        let min_loan_amount = BigUint::from(0u128);

        let new_address = self
            .interactor
            .tx()
            .from(&self.alice_wallet_address)
            .gas(300_000_000u64)
            .typed(proxy::FlashLoanProxy)
            .init(min_loan_amount, basis_points)
            .code(&self.contract_code)
            .code_metadata(CodeMetadata::PAYABLE)
            .returns(ReturnsNewAddress)
            .run()
            .await;
        let new_address_bech32 = bech32::encode(&new_address);
        self.state.set_address(Bech32Address::from_bech32_string(
            new_address_bech32.clone(),
        ));

        println!("new address: {new_address_bech32}");
    }

    pub async fn upgrade(&mut self) {
        let response = self
            .interactor
            .tx()
            .to(self.state.current_address())
            .from(&self.alice_wallet_address)
            .gas(60_000_000u64)
            .typed(proxy::FlashLoanProxy)
            .upgrade()
            .code(&self.contract_code)
            .code_metadata(CodeMetadata::UPGRADEABLE)
            .code_metadata(CodeMetadata::PAYABLE)
            .returns(ReturnsResultUnmanaged)
            .run()
            .await;

        println!("Result: {response:?}");
    }

    pub async fn flash_loan(&mut self, receiver_addr: &str, amount: u128, token_id: String) {
        let loan_token_id = EgldOrEsdtTokenIdentifier::from(token_id.as_bytes());
        let amount_biguint = BigUint::<StaticApi>::from(amount);
        // println!("amount_biguint: {:?}", amount_biguint);

        let loan_receiver_contract_addr = bech32::decode(receiver_addr);
        let receiver_contract_endpoint = ManagedBuffer::new_from_bytes(&b"profitGenerator"[..]);
        let mut args = ManagedArgBuffer::new();
        args.push_arg(BigUint::<StaticApi>::from(0u128));

        let response = self
            .interactor
            .tx()
            .from(&self.alice_wallet_address)
            .to(self.state.current_address())
            .gas(30_000_000u64)
            .typed(proxy::FlashLoanProxy)
            .flash_loan(
                loan_token_id,
                amount_biguint,
                loan_receiver_contract_addr,
                receiver_contract_endpoint,
                args,
            )
            .returns(ReturnsResultUnmanaged)
            .run()
            .await;

        println!("Result: {response:?}");
    }

    pub async fn get_user_pending_fees(&mut self, wallet: &PayerWallet, token_id: &String) {
        let wallet_address = match wallet {
            PayerWallet::Alice => &self.alice_wallet_address,
            PayerWallet::MyWallet => &self.my_wallet_address,
            PayerWallet::Bob => &self.bob_wallet_address,
        };

        let response = self
            .interactor
            .query()
            .to(self.state.current_address())
            .typed(proxy::FlashLoanProxy)
            .get_pending_fees(
                wallet_address,
                EgldOrEsdtTokenIdentifier::from(token_id.as_bytes()),
            )
            .returns(ReturnsResultUnmanaged)
            .run()
            .await;

        println!("Result: {response:?}");
    }

    pub async fn claim_fees(&mut self, token_id: &String, wallet: &PayerWallet) {
        let wallet_address = match wallet {
            PayerWallet::Alice => &self.alice_wallet_address,
            PayerWallet::MyWallet => &self.my_wallet_address,
            PayerWallet::Bob => &self.bob_wallet_address,
        };

        let response = self
            .interactor
            .tx()
            .from(wallet_address)
            .to(self.state.current_address())
            .gas(30_000_000u64)
            .typed(proxy::FlashLoanProxy)
            .claim_fees(EgldOrEsdtTokenIdentifier::from(token_id.as_bytes()))
            .returns(ReturnsResultUnmanaged)
            .run()
            .await;

        println!("Result: {response:?}");
    }

    pub async fn flash_loan_config(&mut self) {
        let min_loan_amount = BigUint::<StaticApi>::from(1_000_000_000_000_000_00u128);
        let fee_percentage_basis_points = 5u32;

        let response = self
            .interactor
            .tx()
            .from(&self.alice_wallet_address)
            .to(self.state.current_address())
            .gas(30_000_000u64)
            .typed(proxy::FlashLoanProxy)
            .flash_loan_config(min_loan_amount, fee_percentage_basis_points)
            .returns(ReturnsResultUnmanaged)
            .run()
            .await;

        println!("Result: {response:?}");
    }

    pub async fn get_max_loan(&mut self, token_id: &String) -> RustBigUint {
        let token_id_clone = token_id.clone();

        let result_value = self
            .interactor
            .query()
            .to(self.state.current_address())
            .typed(proxy::FlashLoanProxy)
            .get_max_loan(EgldOrEsdtTokenIdentifier::from(token_id.as_bytes()))
            .returns(ReturnsResultUnmanaged)
            .run()
            .await;

        result_value
    }

    pub async fn min_loan_amount(&mut self) {
        let result_value = self
            .interactor
            .query()
            .to(self.state.current_address())
            .typed(proxy::FlashLoanProxy)
            .min_loan_amount()
            .returns(ReturnsResultUnmanaged)
            .run()
            .await;

        println!("Result: {result_value:?}");
    }

    pub async fn fee_basis_points(&mut self) {
        let result_value = self
            .interactor
            .query()
            .to(self.state.current_address())
            .typed(proxy::FlashLoanProxy)
            .fee_basis_points()
            .returns(ReturnsResultUnmanaged)
            .run()
            .await;

        println!("Result: {result_value:?}");
    }

    pub async fn add_liquidity(&mut self, token_id: &String, amount: u128, wallet: &PayerWallet) {
        let amount_biguint = BigUint::<StaticApi>::from(amount);
        let wallet_address = match wallet {
            PayerWallet::Alice => &self.alice_wallet_address,
            PayerWallet::MyWallet => &self.my_wallet_address,
            PayerWallet::Bob => &self.bob_wallet_address,
        };

        let token_identifier = match token_id.as_str() {
            "EGLD" => EgldOrEsdtTokenIdentifier::egld(),
            _ => EgldOrEsdtTokenIdentifier::from(token_id.as_bytes()),
        };

        let response = self
            .interactor
            .tx()
            .from(wallet_address)
            .to(self.state.current_address())
            .gas(50_000_000u64)
            .typed(proxy::FlashLoanProxy)
            .add_liquidity()
            .egld_or_single_esdt(&token_identifier, 0, &amount_biguint)
            .returns(ReturnsResultUnmanaged)
            .run()
            .await;

        println!("Result: {response:?}");
    }

    pub async fn withdraw_liquidity(
        &mut self,
        token_id: &String,
        amount: u128,
        wallet: &PayerWallet,
    ) {
        let amount_biguint = BigUint::<StaticApi>::from(amount);

        let token_identifier = EgldOrEsdtTokenIdentifier::from(token_id.as_bytes());

        let payer_wallet = match wallet {
            PayerWallet::Alice => &self.alice_wallet_address,
            PayerWallet::MyWallet => &self.my_wallet_address,
            PayerWallet::Bob => &self.bob_wallet_address,
        };

        let res = self
            .interactor
            .tx()
            .from(payer_wallet)
            .to(self.state.current_address())
            .gas(50_000_000u64)
            .typed(proxy::FlashLoanProxy)
            .withdraw_liquidity(&token_identifier, amount_biguint)
            .returns(ReturnsResultUnmanaged)
            .run()
            .await;

        println!("Result: {res:?}");
    }

    pub async fn get_surplus_balance(&mut self, token_id: &String) -> RustBigUint {
        let res = self
            .interactor
            .query()
            .to(self.state.current_address())
            .typed(proxy::FlashLoanProxy)
            .get_surplus_balance(EgldOrEsdtTokenIdentifier::from(token_id.as_bytes()))
            .returns(ReturnsResultUnmanaged)
            .run()
            .await;

        res
    }

    pub async fn withdraw_surplus(&mut self, token_id: &String, wallet: &PayerWallet) {
        let wallet_address = match wallet {
            PayerWallet::Alice => &self.alice_wallet_address,
            PayerWallet::MyWallet => &self.my_wallet_address,
            PayerWallet::Bob => &self.bob_wallet_address,
        };

        let res = self
            .interactor
            .tx()
            .from(wallet_address)
            .to(self.state.current_address())
            .gas(50_000_000u64)
            .typed(proxy::FlashLoanProxy)
            .withdraw_surplus(EgldOrEsdtTokenIdentifier::from(token_id.as_bytes()))
            .returns(ReturnsResultUnmanaged)
            .run()
            .await;

        println!("Result: {res:?}");
    }

    pub async fn send_tokens_to_contract(
        &mut self,
        token_id: &String,
        amount: u128,
        wallet: &PayerWallet,
    ) {
        let wallet_address = match wallet {
            PayerWallet::Alice => &self.alice_wallet_address,
            PayerWallet::MyWallet => &self.my_wallet_address,
            PayerWallet::Bob => &self.bob_wallet_address,
        };

        let token_identifier = match token_id.as_str() {
            "EGLD" => &EgldOrEsdtTokenIdentifier::egld(),
            _ => &EgldOrEsdtTokenIdentifier::from(token_id.as_bytes()),
        };

        self.interactor
            .tx()
            .from(wallet_address)
            .to(self.state.current_address())
            .gas(50_000_000u64)
            .egld_or_single_esdt(token_identifier, 0, &BigUint::from(amount))
            .run()
            .await;
    }
}
