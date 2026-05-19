use soroban_sdk::{token, Address, Env};

use crate::events::emit_payout_event;
use crate::types::{Circle, ContractError, DataKey};

pub fn process_deposit(env: &Env, member: Address) -> Result<(), ContractError> {
    member.require_auth();

    let circle: Circle = env
        .storage()
        .instance()
        .get(&DataKey::Circle)
        .ok_or(ContractError::NotInitialized)?;

    if !circle.is_active {
        return Err(ContractError::CycleInactive);
    }

    // Member must be registered
    if !env
        .storage()
        .instance()
        .has(&DataKey::MemberSlot(member.clone()))
    {
        return Err(ContractError::NotRegistered);
    }

    // Guard against double-deposit in the same round
    if env
        .storage()
        .instance()
        .get::<DataKey, bool>(&DataKey::Deposit(member.clone(), circle.current_round))
        .unwrap_or(false)
    {
        return Err(ContractError::AlreadyDeposited);
    }

    // Pull tokens from member into contract escrow
    let token_client = token::Client::new(env, &circle.token_address);
    token_client.transfer(
        &member,
        &env.current_contract_address(),
        &circle.contribution_amount,
    );

    // Record deposit for this round
    env.storage().instance().set(
        &DataKey::Deposit(member.clone(), circle.current_round),
        &true,
    );

    Ok(())
}

pub fn draw_pot(env: &Env, caller: Address) -> Result<(), ContractError> {
    caller.require_auth();

    let mut circle: Circle = env
        .storage()
        .instance()
        .get(&DataKey::Circle)
        .ok_or(ContractError::NotInitialized)?;

    if !circle.is_active {
        return Err(ContractError::CycleInactive);
    }

    let total_slots = circle.total_slots;

    // Verify every member has deposited for the current round
    for slot in 0..total_slots {
        let recipient: Address = env
            .storage()
            .instance()
            .get(&DataKey::RoundRecipient(slot))
            .ok_or(ContractError::NotInitialized)?;

        let deposited = env
            .storage()
            .instance()
            .get::<DataKey, bool>(&DataKey::Deposit(recipient, circle.current_round))
            .unwrap_or(false);

        if !deposited {
            return Err(ContractError::EarlyDrawAttempt);
        }
    }

    // Determine this round's recipient
    let recipient: Address = env
        .storage()
        .instance()
        .get(&DataKey::RoundRecipient(circle.current_round))
        .ok_or(ContractError::NotInitialized)?;

    // Compute and transfer the full pool
    let payout = circle
        .contribution_amount
        .checked_mul(total_slots as i128)
        .expect("overflow");

    let token_client = token::Client::new(env, &circle.token_address);
    token_client.transfer(&env.current_contract_address(), &recipient, &payout);

    emit_payout_event(env, &recipient, circle.current_round, payout);

    // Advance round; deactivate if cycle is complete
    circle.current_round = circle.current_round.checked_add(1).expect("overflow");
    if circle.current_round >= total_slots {
        circle.is_active = false;
    }

    env.storage().instance().set(&DataKey::Circle, &circle);

    Ok(())
}
