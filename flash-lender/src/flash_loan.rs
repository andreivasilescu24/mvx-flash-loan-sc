#![no_std]

#[allow(unused_imports)]
use multiversx_sc::imports::*;

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
        require!(
            amount >= self.min_loan_amount().get(),
            "Loan amount is below minimum"
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

        // check if paid back
        self.check_flash_loan_repayment(&back_transfers, loan_token_id, &amount);

        // Emit flash loan event
        self.flash_loan_executed_event(loan_receiver_contract_addr, loan_token_id, &amount);
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

    #[endpoint(addLiquidity)]
    #[payable("*")]
    fn add_liquidity(&self) {
        let payment = self.call_value().egld_or_single_esdt();
        let caller = self.blockchain().get_caller();

        require!(
            payment.amount > BigUint::zero(),
            "Payment amount must be greater than 0"
        );

        let token_id = &payment.token_identifier;

        // Claim pending fees before adding more liquidity
        if !self.user_liquidity_amount(&caller, token_id).is_empty() {
            self.claim_fees_internal(&caller, token_id);
        }

        // Add to user's liquidity
        self.user_liquidity_amount(&caller, token_id)
            .update(|amount| *amount += &payment.amount);

        // Update fee debt: user should only be liable for fees accumulated
        // from this point forward on their entire position
        let user_amount = self.user_liquidity_amount(&caller, token_id).get();
        let accumulated_fees_per_share = self.accumulated_fees_per_share(token_id).get();
        let new_fee_debt =
            &user_amount * &accumulated_fees_per_share / &BigUint::from(1_000_000_000u64);

        self.user_fee_debt(&caller, token_id).set(new_fee_debt);

        // Update total liquidity for this token
        self.total_liquidity(token_id)
            .update(|total| *total += &payment.amount);

        // Add to user's token list if not already present
        if !self.user_liquidity_tokens(&caller).contains(token_id) {
            self.user_liquidity_tokens(&caller).insert(token_id.clone());
        }

        // Emit liquidity added event
        self.liquidity_added_event(&caller, token_id, &payment.amount);
    }

    #[endpoint(withdrawLiquidity)]
    fn withdraw_liquidity(&self, token_id: &EgldOrEsdtTokenIdentifier, amount: BigUint) {
        let caller = self.blockchain().get_caller();
        let user_liquidity = self.user_liquidity_amount(&caller, token_id).get();

        require!(
            amount > BigUint::zero(),
            "Withdrawal amount must be greater than 0"
        );

        require!(amount <= user_liquidity, "Insufficient liquidity balance");

        // Claim pending fees before withdrawal
        self.claim_fees_internal(&caller, token_id);

        // Update user's liquidity
        self.user_liquidity_amount(&caller, token_id)
            .update(|liquidity| *liquidity -= &amount);

        // Update total liquidity
        self.total_liquidity(token_id)
            .update(|total| *total -= &amount);

        // Update fee debt for remaining liquidity
        let remaining_amount = self.user_liquidity_amount(&caller, token_id).get();
        let accumulated_fees_per_share = self.accumulated_fees_per_share(token_id).get();
        let new_fee_debt =
            &remaining_amount * &accumulated_fees_per_share / &BigUint::from(1_000_000_000u64);

        self.user_fee_debt(&caller, token_id).set(new_fee_debt);

        // Remove token from user's list if no liquidity left
        if remaining_amount == BigUint::zero() {
            self.user_liquidity_tokens(&caller).swap_remove(token_id);
        }

        // Send tokens back to user
        self.send().direct(&caller, token_id, 0, &amount);

        // Emit withdraw liquidity event
        self.liquidity_withdrawn_event(&caller, token_id, &amount);
    }

    #[endpoint(claimFees)]
    fn claim_fees(&self, token_id: &EgldOrEsdtTokenIdentifier) {
        let caller = self.blockchain().get_caller();

        // Update pending fees first
        self.claim_fees_internal(&caller, token_id);

        let pending_fees = self.user_pending_fees(&caller, token_id).get();

        require!(pending_fees > BigUint::zero(), "No fees to claim");

        // Reset pending fees
        self.user_pending_fees(&caller, token_id).clear();

        // Track total fees claimed
        self.total_fees_claimed(token_id)
            .update(|total| *total += &pending_fees);

        // Update user_fee_debt to the current accumulated fees per share
        // This ensures that getPendingFees returns 0 after claiming
        let user_amount = self.user_liquidity_amount(&caller, token_id).get();
        let accumulated_fees_per_share = self.accumulated_fees_per_share(token_id).get();
        let precision = BigUint::from(1_000_000_000u64);
        let new_fee_debt = &user_amount * &accumulated_fees_per_share / &precision;

        self.user_fee_debt(&caller, token_id).set(new_fee_debt);

        // Send fees to user
        self.send().direct(&caller, token_id, 0, &pending_fees);

        // Emit claim fees event
        self.fees_claimed_event(&caller, token_id, &pending_fees);
    }

    #[endpoint(withdrawSurplus)]
    #[only_owner]
    fn withdraw_surplus(&self, token_id: &EgldOrEsdtTokenIdentifier) {
        let surplus_balance = self.get_surplus_balance(token_id);

        require!(
            surplus_balance > BigUint::zero(),
            "No surplus balance to withdraw"
        );

        self.send().direct(
            &self.blockchain().get_caller(),
            token_id,
            0,
            &surplus_balance,
        );
    }

    #[view(getSurplusBalance)]
    fn get_surplus_balance(&self, token_id: &EgldOrEsdtTokenIdentifier) -> BigUint {
        let contract_balance = self.blockchain().get_sc_balance(&token_id, 0);
        let total_liquidity = self.total_liquidity(token_id).get();
        let total_fees_distributed = self.total_fees_distributed(token_id).get();
        let total_fees_claimed = self.total_fees_claimed(token_id).get();

        // Unclaimed fees = distributed - claimed
        let unclaimed_fees = if total_fees_distributed > total_fees_claimed {
            total_fees_distributed - total_fees_claimed
        } else {
            BigUint::zero()
        };

        // Surplus = contract balance - liquidity - unclaimed fees
        let total_committed = total_liquidity + unclaimed_fees;

        if contract_balance > total_committed {
            contract_balance - total_committed
        } else {
            BigUint::zero()
        }
    }

    #[view(getTotalUnclaimedFees)]
    fn get_total_unclaimed_fees(&self, token_id: &EgldOrEsdtTokenIdentifier) -> BigUint {
        let total_fees_distributed = self.total_fees_distributed(token_id).get();
        let total_fees_claimed = self.total_fees_claimed(token_id).get();

        if total_fees_distributed > total_fees_claimed {
            total_fees_distributed - total_fees_claimed
        } else {
            BigUint::zero()
        }
    }

    #[view(getMaxLoan)]
    fn get_max_loan(&self, token_id: &EgldOrEsdtTokenIdentifier) -> BigUint {
        // Only user-provided liquidity tokens, not all contract balance
        self.total_liquidity(token_id).get()
    }

    #[view(getUserLiquidity)]
    fn get_user_liquidity(
        &self,
        user: &ManagedAddress,
        token_id: &EgldOrEsdtTokenIdentifier,
    ) -> BigUint {
        self.user_liquidity_amount(user, token_id).get()
    }

    #[view(getPendingFees)]
    fn get_pending_fees(
        &self,
        user: &ManagedAddress,
        token_id: &EgldOrEsdtTokenIdentifier,
    ) -> BigUint {
        let user_amount = self.user_liquidity_amount(user, token_id).get();
        if user_amount == BigUint::zero() {
            return BigUint::zero();
        }

        let current_accumulated = self.accumulated_fees_per_share(token_id).get();
        let user_debt = self.user_fee_debt(user, token_id).get();
        let stored_pending = self.user_pending_fees(user, token_id).get();

        let precision = BigUint::from(1_000_000_000u64);
        let calculated_fees = &user_amount * &current_accumulated / &precision;

        if calculated_fees > user_debt {
            stored_pending + (&calculated_fees - &user_debt)
        } else {
            stored_pending
        }
    }

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

        // require!(
        //     repaid_token_value >= &total_repayment,
        //     "Insufficient repayment: required {} {}, received {} {}",
        //     total_repayment,
        //     token_id,
        //     repaid_token_value,
        //     token_id
        // );
        require!(
            repaid_token_value >= &total_repayment,
            "Insufficient repayment"
        );

        // Get the surplus (flash loan fee)
        let paid_fee = repaid_token_value - loaned_amount;

        self.distribute_flash_loan_fee(token_id, &paid_fee);
    }

    fn distribute_flash_loan_fee(&self, token_id: &EgldOrEsdtTokenIdentifier, paid_fee: &BigUint) {
        let total_liquidity = self.total_liquidity(token_id).get();

        if total_liquidity == BigUint::zero() {
            return; // No liquidity providers to distribute to
        }

        if paid_fee > &BigUint::zero() {
            // Distribute fee proportionally to all liquidity providers
            let precision = BigUint::from(1_000_000_000u64);
            let additional_fees_per_share = (paid_fee * &precision) / &total_liquidity;

            self.accumulated_fees_per_share(token_id)
                .update(|accumulated| *accumulated += &additional_fees_per_share);

            // Track total fees ever distributed for this token
            self.total_fees_distributed(token_id)
                .update(|total| *total += paid_fee);
        }
    }

    fn claim_fees_internal(&self, user: &ManagedAddress, token_id: &EgldOrEsdtTokenIdentifier) {
        let user_amount = self.user_liquidity_amount(user, token_id).get();
        let accumulated_fees_per_share = self.accumulated_fees_per_share(token_id).get();
        let user_fee_debt = self.user_fee_debt(user, token_id).get();

        let precision = BigUint::from(1_000_000_000u64);
        let pending_fees = &user_amount * &accumulated_fees_per_share / &precision;

        if pending_fees > user_fee_debt {
            let fees_to_claim = &pending_fees - &user_fee_debt;

            if fees_to_claim > BigUint::zero() {
                self.user_pending_fees(user, token_id)
                    .update(|pending| *pending += &fees_to_claim);
            }
        }
    }

    #[view(getMinLoan)]
    #[storage_mapper("minLoanAmount")]
    fn min_loan_amount(&self) -> SingleValueMapper<BigUint>;

    // should provide the fee in basis points
    // 1 basis point = 0.01%
    #[view(getFeeBasisPoints)]
    #[storage_mapper("feeBasisPoints")]
    fn fee_basis_points(&self) -> SingleValueMapper<ManagedDecimal<Self::Api, NumDecimals>>;

    // Liquidity tracking
    #[storage_mapper("userLiquidityAmount")]
    fn user_liquidity_amount(
        &self,
        user: &ManagedAddress,
        token_id: &EgldOrEsdtTokenIdentifier,
    ) -> SingleValueMapper<BigUint>;

    #[storage_mapper("totalLiquidity")]
    fn total_liquidity(&self, token_id: &EgldOrEsdtTokenIdentifier) -> SingleValueMapper<BigUint>;

    #[storage_mapper("userLiquidityTokens")]
    fn user_liquidity_tokens(
        &self,
        user: &ManagedAddress,
    ) -> UnorderedSetMapper<EgldOrEsdtTokenIdentifier>;

    #[storage_mapper("accumulatedFeesPerShare")]
    fn accumulated_fees_per_share(
        &self,
        token_id: &EgldOrEsdtTokenIdentifier,
    ) -> SingleValueMapper<BigUint>;

    #[storage_mapper("userFeeDebt")]
    fn user_fee_debt(
        &self,
        user: &ManagedAddress,
        token_id: &EgldOrEsdtTokenIdentifier,
    ) -> SingleValueMapper<BigUint>;

    #[storage_mapper("userPendingFees")]
    fn user_pending_fees(
        &self,
        user: &ManagedAddress,
        token_id: &EgldOrEsdtTokenIdentifier,
    ) -> SingleValueMapper<BigUint>;

    #[storage_mapper("totalFeesDistributed")]
    fn total_fees_distributed(
        &self,
        token_id: &EgldOrEsdtTokenIdentifier,
    ) -> SingleValueMapper<BigUint>;

    #[storage_mapper("totalFeesClaimed")]
    fn total_fees_claimed(
        &self,
        token_id: &EgldOrEsdtTokenIdentifier,
    ) -> SingleValueMapper<BigUint>;

    #[event("flash_loan_executed")]
    fn flash_loan_executed_event(
        &self,
        #[indexed] borrower: &ManagedAddress,
        #[indexed] token_id: &EgldOrEsdtTokenIdentifier,
        amount: &BigUint,
    );

    #[event("liquidity_added")]
    fn liquidity_added_event(
        &self,
        #[indexed] user: &ManagedAddress,
        #[indexed] token_id: &EgldOrEsdtTokenIdentifier,
        amount: &BigUint,
    );

    #[event("liquidity_withdrawn")]
    fn liquidity_withdrawn_event(
        &self,
        #[indexed] user: &ManagedAddress,
        #[indexed] token_id: &EgldOrEsdtTokenIdentifier,
        amount: &BigUint,
    );

    #[event("fees_claimed")]
    fn fees_claimed_event(
        &self,
        #[indexed] user: &ManagedAddress,
        #[indexed] token_id: &EgldOrEsdtTokenIdentifier,
        amount: &BigUint,
    );
}
