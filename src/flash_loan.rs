#![no_std]

#[allow(unused_imports)]
use multiversx_sc::imports::*;

const FLASH_LOAN_PERCENTAGE: u128 = 1000;

#[multiversx_sc::contract]
pub trait FlashLoan {
    #[init]
    fn init(&self) {}

    #[upgrade]
    fn upgrade(&self) {}

    #[endpoint(flashLoan)]
    fn flash_loan(
        &self,
        loan_token_id: &EgldOrEsdtTokenIdentifier,
        amount: BigUint,
        loan_receiver_contract_addr: &ManagedAddress,
        receiver_contract_endpoint: &ManagedBuffer<Self::Api>,
        args: ManagedArgBuffer<Self::Api>,
    ) {
    }
}
