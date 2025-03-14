#![no_std]

#[allow(unused_imports)]
use multiversx_sc::imports::*;

const FLASH_LOAN_PERCENTAGE: u128 = 1000;

/// An empty contract. To be used as a template when starting a new contract from scratch.
#[multiversx_sc::contract]
pub trait FlashLoan {
    #[init]
    fn init(&self) {}

    #[upgrade]
    fn upgrade(&self) {}
}
