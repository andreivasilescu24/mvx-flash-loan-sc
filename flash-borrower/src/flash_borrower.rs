#![no_std]

#[allow(unused_imports)]
use multiversx_sc::imports::*;
use multiversx_sc::{chain_core::types::Address, hex_literal::hex};

use profit_maker::profit_maker_proxy::ProfitMakerProxy;

const FEE_BASIS_POINTS: u128 = 1000;

#[multiversx_sc::contract]
pub trait FlashBorrower {
    #[init]
    fn init(&self) {}

    #[upgrade]
    fn upgrade(&self) {}

    #[endpoint(configProfitGeneratorAddress)]
    fn config_profit_generator_address(&self, profit_generator_address: &ManagedAddress) {
        self.profit_generator_address()
            .set(profit_generator_address);
    }

    #[payable("*")]
    #[endpoint(profitGenerator)]
    fn profit_generator(&self, _arg: BigUint) {
        let mut payment = self.call_value().egld_or_single_esdt();
        let lender = self.blockchain().get_caller();

        require!(
            !self.profit_generator_address().is_empty(),
            "Profit generator address not set"
        );

        self.tx()
            .to(self.profit_generator_address().get())
            .typed(ProfitMakerProxy)
            .take_profit()
            .payment(payment.clone())
            .sync_call();

        // Calculate the fee based on the payment amount
        payment.amount += payment
            .amount
            .clone()
            .mul(BigUint::from(FEE_BASIS_POINTS))
            .div(BigUint::from(10_000u128));

        // repay the loan + fees
        self.tx().to(&lender).payment(payment).transfer();
    }

    #[storage_mapper("profitGeneratorAddress")]
    fn profit_generator_address(&self) -> SingleValueMapper<ManagedAddress>;
}
