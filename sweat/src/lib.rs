use near_contract_standards::fungible_token::events::{FtBurn, FtMint};
use near_contract_standards::fungible_token::metadata::{
    FungibleTokenMetadata, FungibleTokenMetadataProvider,
};
use near_contract_standards::fungible_token::FungibleToken;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::UnorderedSet;
use near_sdk::json_types::{U128, U64};
mod math;

use near_sdk::{env, near_bindgen, AccountId, Balance, PanicOnDefault, PromiseOrValue};

#[near_bindgen]
#[derive(BorshSerialize, BorshDeserialize, PanicOnDefault)]
pub struct Contract {
    oracles: UnorderedSet<AccountId>,
    token: FungibleToken,
    steps_from_tge: U64,
}

#[near_bindgen]
impl Contract {
    #[init]
    pub fn new() -> Self {
        Self {
            oracles: UnorderedSet::new(b"s"),
            token: FungibleToken::new(b"t"),
            steps_from_tge: U64::from(0),
        }
    }
    #[private]
    pub fn add_oracle(&mut self, account_id: &AccountId) {
        assert_eq!(env::predecessor_account_id(), env::current_account_id());
        self.oracles.insert(account_id);
    }

    #[private]
    pub fn remove_oracle(&mut self, account_id: &AccountId) {
        assert_eq!(env::predecessor_account_id(), env::current_account_id());
        self.oracles.remove(account_id);
    }

    pub fn get_oracles(&self) -> Vec<AccountId> {
        self.oracles.to_vec()
    }

    #[private]
    pub fn tge_mint(&mut self, account_id: &AccountId, amount: U128) {
        assert_eq!(env::predecessor_account_id(), env::current_account_id());
        internal_deposit(&mut self.token, &account_id, amount.0);
    }

    #[private]
    pub fn tge_mint_batch(&mut self, batch: Vec<(AccountId, U128)>) {
        assert_eq!(env::predecessor_account_id(), env::current_account_id());
        let mut events = Vec::with_capacity(batch.len());
        for i in 0..batch.len() {
            internal_deposit(&mut self.token, &batch[i].0, batch[i].1 .0);
            events.push(FtMint {
                owner_id: &batch[i].0,
                amount: &batch[i].1,
                memo: None,
            })
        }
        if !events.is_empty() {
            FtMint::emit_many(events.as_slice());
        }
    }

    pub fn burn(&mut self, amount: &U128) {
        self.token
            .internal_withdraw(&env::predecessor_account_id(), amount.0);
        FtBurn {
            owner_id: &env::predecessor_account_id(),
            amount: amount,
            memo: None,
        }
        .emit()
    }

    pub fn get_steps_from_tge(&self) -> U64 {
        self.steps_from_tge
    }

    pub fn record_batch(&mut self, steps_batch: Vec<(AccountId, u16)>) {
        assert!(self.oracles.contains(&env::predecessor_account_id()));
        let mut oracle_fee: U128 = U128(0);
        let mut sweats: Vec<U128> = Vec::with_capacity(steps_batch.len() + 1);
        let mut events = Vec::with_capacity(steps_batch.len() + 1);
        for i in 0..steps_batch.len() {
            let sweat_to_mint: u128 = self.formula(self.steps_from_tge, steps_batch[i].1).0;
            let trx_oracle_fee: u128 = sweat_to_mint * 5 / 100;
            let minted_to_user: u128 = sweat_to_mint - trx_oracle_fee;
            oracle_fee.0 = oracle_fee.0 + trx_oracle_fee;
            internal_deposit(&mut self.token, &steps_batch[i].0, minted_to_user);
            sweats.push(U128(minted_to_user));
            self.steps_from_tge.0 += steps_batch[i].1 as u64;
        }
        for i in 0..steps_batch.len() {
            events.push(FtMint {
                owner_id: &steps_batch[i].0,
                amount: &sweats[i],
                memo: None,
            });
        }
        internal_deposit(
            &mut self.token,
            &env::predecessor_account_id(),
            oracle_fee.0,
        );
        let oracle_event = FtMint {
            owner_id: &env::predecessor_account_id(),
            amount: &oracle_fee,
            memo: None,
        };
        events.push(oracle_event);
        FtMint::emit_many(events.as_slice());
    }

    pub fn formula(&self, steps_from_tge: U64, steps: u16) -> U128 {
        U128(math::formula(steps_from_tge.0 as f64, steps as f64))
    }
}

near_contract_standards::impl_fungible_token_core!(Contract, token);
near_contract_standards::impl_fungible_token_storage!(Contract, token);

/// Taken from contract standards but modified to default if account isn't initialized
/// rather than panicking:
/// <https://github.com/near/near-sdk-rs/blob/6596dc311036fe51d94358ac8f6497ef6e5a7cfc/near-contract-standards/src/fungible_token/core_impl.rs#L105>
fn internal_deposit(token: &mut FungibleToken, account_id: &AccountId, amount: Balance) {
    let balance = token.accounts.get(account_id).unwrap_or_default();
    let new_balance = balance
        .checked_add(amount)
        .unwrap_or_else(|| env::panic_str("Balance overflow"));
    token.accounts.insert(account_id, &new_balance);
    token.total_supply = token
        .total_supply
        .checked_add(amount)
        .unwrap_or_else(|| env::panic_str("Total supply overflow"));
}

pub const ICON: &str = "data:image/svg+xml,%3Csvg viewBox='0 0 100 100' fill='none' xmlns='http://www.w3.org/2000/svg'%3E%3Crect width='100' height='100' rx='50' fill='%23FF0D75'/%3E%3Cg clip-path='url(%23clip0_283_2788)'%3E%3Cpath d='M39.4653 77.5455L19.0089 40.02L35.5411 22.2805L55.9975 59.806L39.4653 77.5455Z' stroke='white' stroke-width='10'/%3E%3Cpath d='M66.0253 77.8531L45.569 40.3276L62.1012 22.5882L82.5576 60.1136L66.0253 77.8531Z' stroke='white' stroke-width='10'/%3E%3C/g%3E%3Cdefs%3E%3CclipPath id='clip0_283_2788'%3E%3Crect width='100' height='56' fill='white' transform='translate(0 22)'/%3E%3C/clipPath%3E%3C/defs%3E%3C/svg%3E%0A";

#[near_bindgen]
impl FungibleTokenMetadataProvider for Contract {
    fn ft_metadata(&self) -> FungibleTokenMetadata {
        FungibleTokenMetadata {
            spec: "ft-1.0".to_string(),
            name: "SWEAT".to_string(),
            symbol: "SWEAT".to_string(),
            icon: Some(String::from(ICON)),
            reference: None,
            reference_hash: None,
            decimals: 18,
        }
    }
}
