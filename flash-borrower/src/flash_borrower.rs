#![no_std]

#[allow(unused_imports)]
use multiversx_sc::imports::*;

pub mod flash_borrower_proxy;
// const FEE_PERCENTAGE: u128 =

/// An empty contract. To be used as a template when starting a new contract from scratch.
#[multiversx_sc::contract]
pub trait FlashBorrower {
    #[init]
    fn init(&self) {}

    #[upgrade]
    fn upgrade(&self) {}

    #[payable("EGLD")]
    #[endpoint(flash)]
    fn flash(&self) {
        let payment = self.call_value().egld_or_single_esdt();
        let lender = self.blockchain().get_caller();

        // let repayment = payment.amount.mul(FLAS)
    }
}
