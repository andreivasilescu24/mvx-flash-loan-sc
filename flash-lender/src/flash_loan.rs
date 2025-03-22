#![no_std]

#[allow(unused_imports)]
use multiversx_sc::imports::*;
use multiversx_sc::storage;

#[multiversx_sc::contract]
pub trait FlashLoan {
    #[init]
    fn init(&self, min_loan_amount: BigUint, fee_percentage: BigUint) {
        self.min_loan_amount().set(min_loan_amount);
        self.fee_percentage().set(fee_percentage);
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

        // set ongoing flash loan
        // loan tx
        // check if paid back
    }

    #[endpoint(flashLoanConfig)]
    #[only_owner]
    fn flash_loan_config(&self, min_loan_amount: BigUint, fee_percentage: BigUint) {
        self.min_loan_amount().set(min_loan_amount);
        self.fee_percentage().set(fee_percentage);
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

    #[view(getMaxLoan)]
    fn get_max_loan(&self) -> BigUint {
        self.blockchain()
            .get_sc_balance(&EgldOrEsdtTokenIdentifier::egld(), 0)
    }

    #[view(getMinLoan)]
    #[storage_mapper("minLoanAmount")]
    fn min_loan_amount(&self) -> SingleValueMapper<BigUint>;

    #[view(getFeePercentage)]
    #[storage_mapper("feePercentage")]
    fn fee_percentage(&self) -> SingleValueMapper<BigUint>;
}
