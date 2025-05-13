#![no_std]

#[allow(unused_imports)]
use multiversx_sc::imports::*;

pub mod flash_borrower_proxy;
const FEE_BASIS_POINTS: u128 = 5;

/// An empty contract. To be used as a template when starting a new contract from scratch.
#[multiversx_sc::contract]
pub trait FlashBorrower {
    #[init]
    fn init(&self) {}

    #[upgrade]
    fn upgrade(&self) {}

    #[payable("*")]
    #[endpoint(flash)]
    fn flash(&self, arg: BigUint) {
        let mut payment = self.call_value().egld_or_single_esdt();
        let lender = self.blockchain().get_caller();

        // let received_loan = ManagedDecimal::from_raw_units(payment.amount, 0);
        // let fee_percentage = ManagedDecimal::from_raw_units(BigUint::from(FEE_BASIS_POINTS), 4);
        // let fee = received_loan.clone().mul(fee_percentage.clone());
        // let repay_amount = received_loan.clone().add(fee.clone());

        payment.amount += payment
            .amount
            .clone()
            .mul(BigUint::from(FEE_BASIS_POINTS))
            .div(BigUint::from(1000u128));

        // let token_id = payment.token_identifier;

        self.tx().to(&lender).payment(payment).transfer();

        // self.tx()
        //     .to(&lender)
        //     .payment(EgldOrEsdtTokenPayment::new(
        //         token_id,
        //         0,
        //         repay_amount.into_raw_units().clone(),
        //     ))
        //     .transfer();

        // let repayment = payment.amount.mul(FLAS)
    }
}
