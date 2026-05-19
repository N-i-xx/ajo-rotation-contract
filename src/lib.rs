use soroban_sdk::{contract, contractimpl, Address, Env};

pub mod events;
pub mod lifecycle;
pub mod pools;
pub mod types;

use types::ContractError;

pub struct AjoContract;

#[contractimpl]
impl AjoContract {
    /// One-time setup: configure token, contribution amount, and number of members.
    pub fn initialize_circle(
        env: Env,
        token_address: Address,
        contribution_amount: i128,
        total_slots: u32,
    ) -> Result<(), ContractError> {
        lifecycle::initialize_circle(&env, token_address, contribution_amount, total_slots)
    }

    /// Register a member and assign them a rotation slot.
    pub fn register_member(env: Env, member: Address) -> Result<(), ContractError> {
        lifecycle::register_member(&env, member)
    }

    /// Pull contribution_amount tokens from member into contract escrow.
    pub fn process_deposit(env: Env, member: Address) -> Result<(), ContractError> {
        pools::process_deposit(&env, member)
    }

    /// Verify all deposits, pay out the current round's recipient, advance the round.
    pub fn draw_pot(env: Env, caller: Address) -> Result<(), ContractError> {
        pools::draw_pot(&env, caller)
    }
}
