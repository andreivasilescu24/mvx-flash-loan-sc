#![no_std]

use core::arch::x86_64;

#[allow(unused_imports)]
use multiversx_sc::imports::*;
use multiversx_sc::storage;

use flash_borrower::flash_borrower_proxy::FlashBorrowerProxy;

const NUM_DECIMALS: usize = 4;

#[multiversx_sc::contract]
pub trait FlashLoan {
    #[init]
    fn init(&self, min_loan_amount: BigUint, fee_percentage_basis_points: u32) {
        self.min_loan_amount().set(min_loan_amount);

        let fee_percentage = ManagedDecimal::from_raw_units(
            BigUint::from(fee_percentage_basis_points),
            NUM_DECIMALS,
        );

        self.fee_basis_points().set(fee_percentage);
    }

    #[upgrade]
    fn upgrade(&self) {}

    #[endpoint(flashLoan)]
    fn flash_loan(
        &self,
        loan_token_id: &EgldOrEsdtTokenIdentifier,
        amount: BigUint,
        loan_receiver_contract_addr: &ManagedAddress,
        receiver_contract_endpoint: ManagedBuffer<Self::Api>,
        args: ManagedArgBuffer<Self::Api>,
    ) {
        require!(
            amount > BigUint::from(0u128),
            "Loaned amount must be greater than 0"
        );
        self.check_contract_shard(loan_receiver_contract_addr);
        self.check_loan_amount_available(&amount);

        let initial_balance = self.blockchain().get_sc_balance(loan_token_id, 0);

        // loan tx
        let loaned_amount = amount.clone();

        let back_transfers = self
            .tx()
            .to(loan_receiver_contract_addr)
            .typed(FlashBorrowerProxy)
            .flash()
            .egld(&amount.into())
            .returns(ReturnsBackTransfers)
            .sync_call();

        // check if paid back
        self.check_flash_loan_repayment(&back_transfers, &loaned_amount);
    }

    #[endpoint(flashLoanConfig)]
    #[only_owner]
    fn flash_loan_config(&self, min_loan_amount: BigUint, fee_percentage_basis_points: u32) {
        self.min_loan_amount().set(min_loan_amount);

        let fee_percentage = ManagedDecimal::from_raw_units(
            BigUint::from(fee_percentage_basis_points),
            NUM_DECIMALS,
        );

        self.fee_basis_points().set(fee_percentage);
    }

    #[endpoint(repayLoan)]
    #[payable("EGLD")] // for the moment
    fn repay_loan(&self) {}

    fn check_contract_shard(&self, contract_addr: &ManagedAddress) {
        let my_contract_addr = self.blockchain().get_sc_address();
        require!(
            self.blockchain().get_shard_of_address(contract_addr)
                == self.blockchain().get_shard_of_address(&my_contract_addr),
            "Contract is not in the same shard"
        );
    }

    fn check_loan_amount_available(&self, amount: &BigUint) {
        require!(amount <= &self.get_max_loan(), "Not enough funds available");
    }

    fn compute_loan_repayment_amount(&self) -> BigUint {
        BigUint::from(0u128)
    }

    fn check_flash_loan_repayment(
        &self,
        loan_back_transfers: &BackTransfers<Self::Api>,
        loaned_amount: &BigUint,
    ) {
        let repaid_egld_value = &loan_back_transfers.total_egld_amount;

        let fee_percentage_basis_points = self.fee_basis_points().get();
        let loaned_amount_decimal =
            ManagedDecimal::from_raw_units(loaned_amount.clone(), NUM_DECIMALS);

        // compute total egld value that should be repaid

        // require repaid >= expected
    }

    #[view(getMaxLoan)]
    fn get_max_loan(&self) -> BigUint {
        self.blockchain()
            .get_sc_balance(&EgldOrEsdtTokenIdentifier::egld(), 0)
    }

    #[view(getMinLoan)]
    #[storage_mapper("minLoanAmount")]
    fn min_loan_amount(&self) -> SingleValueMapper<BigUint>;

    // should provide the fee in basis points
    // 1 basis point = 0.01%
    #[view(getFeeBasisPoints)]
    #[storage_mapper("feeBasisPoints")]
    fn fee_basis_points(&self) -> SingleValueMapper<ManagedDecimal<Self::Api, NumDecimals>>;
}
