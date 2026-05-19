use soroban_sdk::{symbol_short, Address, Env};

pub fn emit_payout_event(env: &Env, recipient: &Address, round: u32, amount: i128) {
    env.events().publish(
        (symbol_short!("payout"),),
        (recipient.clone(), round, amount),
    );
}
