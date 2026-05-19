use soroban_sdk::{Address, Env};

use crate::types::{Circle, ContractError, DataKey};

pub fn initialize_circle(
    env: &Env,
    token_address: Address,
    contribution_amount: i128,
    total_slots: u32,
) -> Result<(), ContractError> {
    if env.storage().instance().has(&DataKey::Circle) {
        return Err(ContractError::AlreadyInitialized);
    }

    let circle = Circle {
        contribution_amount,
        current_round: 0,
        total_slots,
        token_address,
        is_active: true,
    };

    env.storage().instance().set(&DataKey::Circle, &circle);
    env.storage().instance().set(&DataKey::MemberCount, &0u32);

    Ok(())
}

pub fn register_member(env: &Env, member: Address) -> Result<(), ContractError> {
    member.require_auth();

    let circle: Circle = env
        .storage()
        .instance()
        .get(&DataKey::Circle)
        .ok_or(ContractError::NotInitialized)?;

    if !circle.is_active {
        return Err(ContractError::CycleInactive);
    }

    if env
        .storage()
        .instance()
        .has(&DataKey::MemberSlot(member.clone()))
    {
        return Err(ContractError::AlreadyRegistered);
    }

    let count: u32 = env
        .storage()
        .instance()
        .get(&DataKey::MemberCount)
        .unwrap_or(0);

    if count >= circle.total_slots {
        return Err(ContractError::CycleFull);
    }

    // Assign slot = current count (0-based join order)
    env.storage()
        .instance()
        .set(&DataKey::MemberSlot(member.clone()), &count);

    // Record who receives the pot in round `count`
    env.storage()
        .instance()
        .set(&DataKey::RoundRecipient(count), &member);

    env.storage()
        .instance()
        .set(&DataKey::MemberCount, &(count + 1));

    Ok(())
}
