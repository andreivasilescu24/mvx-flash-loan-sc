use clap::builder::Str;
use multiversx_sc_snippets::imports::*;
use rust_interact::{config::Config, ContractInteract};

// Simple deploy test that runs on the real blockchain configuration.
// In order for this test to work, make sure that the `config.toml` file contains the real blockchain config (or choose it manually)
// Can be run with `sc-meta test`.
// const esdt_token_id: String = String::from("FLT-03f83f");

#[tokio::test]
#[ignore = "run on demand, relies on real blockchain state"]
async fn deploy_test_flash_loan() {
    let mut interactor = ContractInteract::new(Config::new()).await;

    interactor.deploy().await;
}

#[tokio::test]
async fn test_upgrade_sc() {
    let mut interactor = ContractInteract::new(Config::new()).await;

    interactor.upgrade().await;
}

#[tokio::test]
async fn test_flash_config() {
    let mut interactor = ContractInteract::new(Config::new()).await;

    interactor.flash_loan_config().await;
    print!("Min loan amount: ");
    interactor.min_loan_amount().await;
    print!("Fees: ");
    interactor.fee_basis_points().await;
}

#[tokio::test]
async fn test_flash_loan_scenario() {
    let mut interactor = ContractInteract::new(Config::new()).await;

    let loan_amount = 2000000000000000000u128; // 2 egld/test esdt token
    let receiver_contract_addr = "erd1qqqqqqqqqqqqqpgq0ugg5w5ks6kq60c03j4qy8yxylxpdh6hd8ssvl3e3d";
    let token_id = String::from("FLT-03f83f");

    interactor.get_max_loan(&token_id).await;

    let token_id_clone = token_id.clone();

    interactor
        .flash_loan(receiver_contract_addr, loan_amount, token_id)
        .await;

    interactor.get_max_loan(&token_id_clone).await;

    println!("Flash loan executed successfully");
}

#[tokio::test]
async fn test_max_loan() {
    let token_id = String::from("FLT-03f83f");
    let mut interactor = ContractInteract::new(Config::new()).await;
    interactor.get_max_loan(&token_id).await;
}

// ManagedDecimal tests
#[tokio::test]
async fn test_loan_amount() {
    let loaned_amount = ManagedDecimal::<StaticApi, NumDecimals>::from_raw_units(
        BigUint::from(1_000_000_000_000_000_000u128),
        4,
    );

    let fee_percentage =
        ManagedDecimal::<StaticApi, NumDecimals>::from_raw_units(BigUint::from(5u32), 4);

    let repay_amount = loaned_amount
        .clone()
        .add(loaned_amount.clone().mul(fee_percentage.clone()));

    println!("Loaned amount: {loaned_amount}");
    println!("Fee percentage: {fee_percentage}");
    println!("Repay amount: {repay_amount}");
}

#[tokio::test]
async fn test_loan_amount_with_real_denominations() {
    let egld = BigUint::from(1_700_000_000_000_000_000_u128);
    let egld_managed_dec =
        ManagedDecimal::<StaticApi, NumDecimals>::from_raw_units(egld.clone(), 0);
    let fee_percentage = BigUint::from(5u32);
    let fee_percentage_managed_dec =
        ManagedDecimal::<StaticApi, NumDecimals>::from_raw_units(fee_percentage.clone(), 4);

    let fee_to_pay = egld_managed_dec
        .clone()
        .mul(fee_percentage_managed_dec.clone());

    let total_amount_to_pay = egld_managed_dec.clone().add(fee_to_pay.clone());
    let total_repayment_big_uint = total_amount_to_pay.trunc();

    println!("EGLD: {:?}", egld);
    println!("EGLD managed decimal: {egld_managed_dec}");
    println!("Fee percentage: {:?}", fee_percentage);
    println!("Fee percentage managed decimal: {fee_percentage_managed_dec}");
    println!("Fee to pay: {fee_to_pay}");
    println!("Total amount to pay: {total_amount_to_pay}");
    println!(
        "Total repayment (BigUint): {:?}",
        total_repayment_big_uint.to_u64()
    );
}
