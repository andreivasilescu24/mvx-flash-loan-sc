#![no_std]

#[allow(unused_imports)]
use multiversx_sc::imports::*;

use flash_borrower::flash_borrower_proxy::FlashBorrowerProxy;

const NUM_DECIMALS: usize = 4;

// TODO:
// - add esdt token support
// - add liquidity for users endpoint + withdraw liquidity + withdraw reward

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
        self.check_loan_amount_available(&amount, loan_token_id);

        let back_transfers = self
            .tx()
            .to(loan_receiver_contract_addr)
            .raw_call(receiver_contract_endpoint)
            .arguments_raw(args)
            .egld_or_single_esdt(loan_token_id, 0, &amount)
            .returns(ReturnsBackTransfersReset)
            .sync_call();

        // back_transfers.esdt_payments.

        // sc_print!("Received {}", back_transfers.total_egld_amount);

        // check if paid back
        self.check_flash_loan_repayment(&back_transfers, loan_token_id, &amount);
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
    #[payable("*")]
    fn repay_loan(&self) {}

    fn check_contract_shard(&self, contract_addr: &ManagedAddress) {
        let my_contract_addr = self.blockchain().get_sc_address();
        require!(
            self.blockchain().get_shard_of_address(contract_addr)
                == self.blockchain().get_shard_of_address(&my_contract_addr),
            "Contract is not in the same shard"
        );
    }

    fn check_loan_amount_available(
        &self,
        amount: &BigUint,
        loan_token_id: &EgldOrEsdtTokenIdentifier,
    ) {
        require!(
            amount <= &self.get_max_loan(loan_token_id),
            "Not enough balance available for the requested token"
        );
    }

    fn compute_loan_repayment_amount(
        &self,
        loaned_amount: &ManagedDecimal<Self::Api, NumDecimals>,
    ) -> ManagedDecimal<Self::Api, NumDecimals> {
        let fee = loaned_amount.clone().mul(self.fee_basis_points().get());
        loaned_amount.clone().add(fee)
    }

    fn check_flash_loan_repayment(
        &self,
        loan_back_transfers: &BackTransfers<Self::Api>,
        token_id: &EgldOrEsdtTokenIdentifier,
        loaned_amount: &BigUint,
    ) {
        let repaid_token_value = if token_id.is_egld() {
            &loan_back_transfers.total_egld_amount
        } else {
            let repay_esdt_transfers = &loan_back_transfers.esdt_payments;

            require!(
                repay_esdt_transfers.len() == 1,
                "Expected exactly one ESDT payment for repayment"
            );

            let repayment = repay_esdt_transfers.get(0);

            require!(
                repayment.token_identifier == token_id.clone(),
                "Token used for repayment doesn't match the loan token"
            );

            &repayment.clone().amount
        };

        // Convert loaned amount to ManagedDecimal for precision calculations
        let loaned_amount_decimal = ManagedDecimal::from_raw_units(loaned_amount.clone(), 0);

        // Calculate the total repayment amount (principal + fee)
        let total_repayment_decimal = self.compute_loan_repayment_amount(&loaned_amount_decimal);

        // Convert back to BigUint for comparison with the repaid amount
        let total_repayment = total_repayment_decimal.trunc();

        require!(
            repaid_token_value >= &total_repayment,
            "Insufficient repayment: required {} {}, received {} {}",
            total_repayment,
            token_id,
            repaid_token_value,
            token_id
        );
    }

    #[view(getMaxLoan)]
    fn get_max_loan(&self, token_id: &EgldOrEsdtTokenIdentifier) -> BigUint {
        self.blockchain().get_sc_balance(token_id, 0)
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
