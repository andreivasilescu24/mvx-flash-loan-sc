#![no_std]

use multiversx_sc::imports::*;

pub mod profit_maker_proxy;

#[multiversx_sc::contract]
pub trait ProfitMaker {
    #[init]
    fn init(&self, fee_basis_points: BigUint) {
        self.fee_basis_points().set(fee_basis_points);
    }

    #[upgrade]
    fn upgrade(&self) {}

    /// Add desired amount to the storage variable.
    #[payable("*")]
    #[endpoint(takeProfit)]
    fn take_profit(&self) {
        let mut payment = self.call_value().egld_or_single_esdt();
        let caller = self.blockchain().get_caller();

        // Calculate the fee based on the payment amount
        let fee = payment
            .amount
            .clone()
            .mul(self.fee_basis_points().get())
            .div(BigUint::from(10_000u128));

        let profit = fee.clone().mul(BigUint::from(2u128));
        payment.amount += fee;
        payment.amount += profit;

        // Send the payment back to the caller
        self.tx().to(&caller).payment(payment).transfer();
    }

    #[view(getFeeBasisPoints)]
    #[storage_mapper("fee_basis_points")]
    fn fee_basis_points(&self) -> SingleValueMapper<BigUint>;
}
