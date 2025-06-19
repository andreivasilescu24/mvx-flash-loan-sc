use core::time;
use std::time::Duration;

use multiversx_sc_snippets::imports::*;
use rust_interact::{config::Config, ContractInteract, FlashLoanFailScenario, PayerWallet};

// Simple deploy test that runs on the real blockchain configuration.
// In order for this test to work, make sure that the `config.toml` file contains the real blockchain config (or choose it manually)
// Can be run with `sc-meta test`.
// const esdt_token_id: String = String::from("FLT-03f83f");

#[tokio::test]
#[ignore = "run on demand, relies on real blockchain state"]
async fn deploy_test_flash_loan() {
    let mut interactor = ContractInteract::new(Config::new()).await;

    let basis_points = 1000;

    interactor.deploy(basis_points).await;
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

    let loan_amount = 1_000_000_000_000_000_000u128;
    let receiver_contract_addr = "erd1qqqqqqqqqqqqqpgqf7mxaqsvjm96m09w0z0end3ft8s67wcwd8ssh6wjvf";
    let token_id = String::from("EGLD");

    interactor.get_max_loan(&token_id).await;

    interactor
        .flash_loan(receiver_contract_addr, loan_amount, token_id)
        .await;
}

#[tokio::test]
async fn test_multiple_flash_loans() {
    let mut interactor = ContractInteract::new(Config::new()).await;

    let loan_amount = 1_000_000_000_000_000_000u128;
    let receiver_contract_addr = "erd1qqqqqqqqqqqqqpgqf7mxaqsvjm96m09w0z0end3ft8s67wcwd8ssh6wjvf";
    let token_id = String::from("EGLD");

    interactor.get_max_loan(&token_id).await;

    for _ in 0..11 {
        interactor
            .flash_loan(receiver_contract_addr, loan_amount, token_id.clone())
            .await;
        tokio::time::sleep(Duration::from_secs(6)).await;
    }
}

#[tokio::test]
async fn test_flash_loan_failed_scenario() {
    let mut interactor = ContractInteract::new(Config::new()).await;

    let loan_amount = 1_000_000_000_000_000_000u128;
    let receiver_contract_addr = "erd1qqqqqqqqqqqqqpgqf7mxaqsvjm96m09w0z0end3ft8s67wcwd8ssh6wjvf";
    let token_id = String::from("EGLD");

    let flash_loan_fail_scenario = FlashLoanFailScenario::NoRepayment;
    let error_message = ExpectError(4, "Insufficient repayment");

    interactor
        .flash_loan_failed(
            receiver_contract_addr,
            loan_amount,
            token_id,
            flash_loan_fail_scenario,
            error_message,
        )
        .await;
}

#[tokio::test]
async fn test_claim_fees_split_scenario() {
    let mut interactor = ContractInteract::new(Config::new()).await;
    let token_id = String::from("EGLD");
    let tokend_id_clone = token_id.clone();
    let amount = 1_000_000_000_000_000_000;

    let wallet_alice = PayerWallet::Alice;
    let wallet_bob = PayerWallet::Bob;
    let my_wallet = PayerWallet::MyWallet;

    let receiver_contract_addr = "erd1qqqqqqqqqqqqqpgqf7mxaqsvjm96m09w0z0end3ft8s67wcwd8ssh6wjvf";

    interactor
        .add_liquidity(&token_id, amount, &wallet_alice)
        .await;
    interactor
        .add_liquidity(&token_id, amount, &wallet_bob)
        .await;
    interactor
        .add_liquidity(&token_id, amount, &my_wallet)
        .await;

    interactor
        .flash_loan(receiver_contract_addr, amount, token_id)
        .await;

    interactor.claim_fees(&tokend_id_clone, &wallet_alice).await;
    interactor.claim_fees(&tokend_id_clone, &wallet_bob).await;
    interactor.claim_fees(&tokend_id_clone, &my_wallet).await;
}

#[tokio::test]
async fn test_add_liquidity_after_flash_loan_no_fees_received() {
    let mut interactor = ContractInteract::new(Config::new()).await;
    let token_id = String::from("EGLD");
    let token_id_clone = token_id.clone();
    let amount = 1_000_000_000_000_000_000u128; // 1 EGLD
    let receiver_contract_addr = "erd1qqqqqqqqqqqqqpgqf7mxaqsvjm96m09w0z0end3ft8s67wcwd8ssh6wjvf";
    let expected_fee_claim_error = ExpectError(4, "No fees to claim");

    let wallet_alice = PayerWallet::Alice;
    let my_wallet = PayerWallet::MyWallet;
    interactor
        .add_liquidity(&token_id, amount, &wallet_alice)
        .await;
    interactor
        .flash_loan(&receiver_contract_addr, amount, token_id)
        .await;
    interactor
        .add_liquidity(&token_id_clone, amount, &my_wallet)
        .await;

    interactor.claim_fees(&token_id_clone, &wallet_alice).await;
    interactor
        .claim_fees_fail_test(&token_id_clone, &my_wallet, expected_fee_claim_error)
        .await;
}

#[tokio::test]
async fn test_add_liquidity() {
    let mut interactor = ContractInteract::new(Config::new()).await;
    let token_id = String::from("EGLD");
    let amount = 1_000_000_000_000_000_000u128; // 1 EGLD

    let wallet = PayerWallet::Bob;
    interactor.add_liquidity(&token_id, amount, &wallet).await;
}

#[tokio::test]
async fn test_get_user_pending_fees() {
    let mut interactor = ContractInteract::new(Config::new()).await;
    let user_address = PayerWallet::Alice;
    let token_id = String::from("EGLD");
    interactor
        .get_user_pending_fees(&user_address, &token_id)
        .await;
}

#[tokio::test]
async fn test_claim_fees() {
    let mut interactor = ContractInteract::new(Config::new()).await;
    let token_id = String::from("EGLD");
    let wallet = PayerWallet::Alice;
    interactor.claim_fees(&token_id, &wallet).await;
}

#[tokio::test]
async fn test_withdraw_liqudity() {
    let mut interactor = ContractInteract::new(Config::new()).await;
    let amount = 1_000_000_000_000_000_000u128;
    let token_id = String::from("EGLD");
    let wallet = PayerWallet::MyWallet;
    interactor
        .withdraw_liquidity(&token_id, amount, &wallet)
        .await;
}

#[tokio::test]
async fn test_max_loan() {
    let token_id = String::from("EGLD");
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

#[tokio::test]
async fn test_get_surplus_balance() {
    let mut interactor = ContractInteract::new(Config::new()).await;

    let token_id = String::from("EGLD");
    let amount = 1_000_000_000_000_000_000u128;
    let wallet = PayerWallet::Alice;

    let max_loan_before = interactor.get_max_loan(&token_id).await;
    let surplus_balance_before = interactor.get_surplus_balance(&token_id).await;

    println!("Max loan before: {max_loan_before}");
    println!("Surplus balance before: {surplus_balance_before}");

    println!("Adding {} tokens as liquidity", amount);
    interactor.add_liquidity(&token_id, amount, &wallet).await;

    println!("Sending {} tokens to contract", amount);
    interactor
        .send_tokens_to_contract(&token_id, amount, &wallet)
        .await;

    let max_loan_after = interactor.get_max_loan(&token_id).await;
    println!("Max loan after: {max_loan_after}");
    let surplus_balance_after = interactor.get_surplus_balance(&token_id).await;
    println!("Surplus balance after: {surplus_balance_after}");
}

#[tokio::test]
async fn test_withdraw_surplus_balance() {
    let mut interactor = ContractInteract::new(Config::new()).await;
    let token_id = String::from("EGLD");
    let wallet = PayerWallet::Alice;
    interactor.withdraw_surplus(&token_id, &wallet).await;
}
