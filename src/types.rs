use soroban_sdk::{contracttype, Address};

#[contracttype]
#[derive(Clone)]
pub struct Circle {
    pub contribution_amount: i128,
    pub current_round: u32,
    pub total_slots: u32,
    pub token_address: Address,
    pub is_active: bool,
}

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Circle,
    MemberSlot(Address),       // Address → slot index (u32)
    Deposit(Address, u32),     // (member, round) → bool
    RoundRecipient(u32),       // round → Address
    MemberCount,               // u32 — how many members have registered
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ContractError {
    AlreadyInitialized  = 1,
    NotInitialized      = 2,
    CycleFull           = 3,
    AlreadyRegistered   = 4,
    NotRegistered       = 5,
    AlreadyDeposited    = 6,
    EarlyDrawAttempt    = 7,
    CycleInactive       = 8,
}
