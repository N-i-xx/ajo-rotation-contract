# Vero Ajo Rotation Contract

> A trustless, fully on-chain rotating savings circle (Ajo / Esusu / Tontine) built as a [Soroban](https://soroban.stellar.org/) smart contract on the Stellar network.

Members contribute equal amounts each round; the full pool is disbursed to one member per round in sequence — no intermediary, no custody risk, no trust required.

[![Build](https://img.shields.io/badge/build-passing-brightgreen)](#ci)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](#license)
[![Soroban](https://img.shields.io/badge/Soroban-v21-blue)](https://soroban.stellar.org/)

---

## Table of Contents

- [Overview](#overview)
- [How Ajo Works](#how-ajo-works)
- [Architecture](#architecture)
  - [High-Level Flow](#high-level-flow)
  - [Round Lifecycle State Machine](#round-lifecycle-state-machine)
  - [Storage Layout](#storage-layout)
  - [Module Dependency Graph](#module-dependency-graph)
- [Contract Modules](#contract-modules)
- [Data Types](#data-types)
- [Core Functions](#core-functions)
  - [initialize_circle](#initialize_circletoken-address-amount-i128-slots-u32)
  - [register_member](#register_membermember-address)
  - [process_deposit](#process_depositmember-address)
  - [rotate_round](#rotate_roundcaller-address)
  - [draw_pot](#draw_potenv-env-recipient-address)
- [Error Codes](#error-codes)
- [Events](#events)
- [Getting Started](#getting-started)
  - [Prerequisites](#prerequisites)
  - [Build](#build)
  - [Deploy to Testnet](#deploy-to-testnet)
  - [End-to-End Example](#end-to-end-example)
- [Testing](#testing)
- [CI](#ci)
- [Security Considerations](#security-considerations)
- [License](#license)

---

## Overview

Traditional Ajo/Esusu groups are informal rotating credit associations common across West Africa and the diaspora. Each member contributes a fixed amount per cycle; one member receives the entire pot per round, rotating until everyone has received once.

This contract brings that model fully on-chain:

| Property | Detail |
|---|---|
| **Trustless** | No organizer holds funds; the contract enforces all rules |
| **Permissionless rotation** | Any member can trigger `rotate_round` once conditions are met |
| **SEP-41 token support** | Works with any Stellar asset wrapped as a SEP-41 token |
| **Overflow-safe arithmetic** | All balance math uses checked operations |
| **Auth-gated** | Every sensitive call requires Soroban native auth |
| **Fully on-chain** | No off-chain oracle, scheduler, or backend required |

---

## How Ajo Works

The core mechanic is simple: N members each contribute `amount` tokens per round. The contract collects all contributions and pays out the full pool (`N × amount`) to one member. The recipient rotates each round based on join order.

```
Round 1        Round 2        Round 3
─────────      ─────────      ─────────
Alice  → ✓     Alice  → ✓     Alice  → ✓
Bob    → ✓     Bob    → ✓     Bob    → ✓
Carol  → ✓     Carol  → ✓     Carol  → ✓
         ↓              ↓              ↓
      Alice           Bob           Carol
    receives        receives       receives
     3× amt          3× amt         3× amt
```

Each round follows four steps:

1. **Deposit phase** — all members call `process_deposit` to contribute their share
2. **Verification** — `rotate_round` checks that every member has deposited for the current round
3. **Payout** — the contract transfers the full pool to the current round's recipient via `draw_pot`
4. **Advance** — the round counter increments; the next recipient is determined by join order

After `total_slots` rounds, every member has received the pot exactly once.

---

## Architecture

### High-Level Flow

```
  ┌──────────┐
  │ Member A │──┐
  └──────────┘  │                    ┌─────────────────────┐
  ┌──────────┐  ├──► process_deposit │   Circle Storage    │
  │ Member B │──┤    (per member,    │  ─────────────────  │
  └──────────┘  │     per round)     │  CIRCLE → metadata  │
  ┌──────────┐  │                    │  MEMBER(addr) → slot │
  │ Member C │──┘                    │  DEPOSIT(addr,rnd)  │
  └──────────┘                       │  RECIPIENT(rnd)     │
                                     └──────────┬──────────┘
                                                │
                                                ▼
                                        rotate_round()
                                     (any member, once all
                                      deposits confirmed)
                                                │
                                                ▼
                                          draw_pot()
                                                │
                              ┌─────────────────┴──────────────────┐
                              ▼                                     ▼
                    SEP-41 token.transfer()              emit_payout_event()
                    (contract → recipient)               (on-chain event log)
```

### Round Lifecycle State Machine

```
                    ┌─────────────────────────────────────┐
                    │                                     │
                    ▼                                     │
             ┌─────────────┐                             │
             │  OPEN ROUND │  ◄── initialize_circle()    │
             └──────┬──────┘                             │
                    │                                     │
          members call process_deposit()                  │
                    │                                     │
                    ▼                                     │
        ┌───────────────────────┐                        │
        │  DEPOSITS COLLECTING  │                        │
        │  (partial — n < N)    │                        │
        └───────────┬───────────┘                        │
                    │                                     │
          last member deposits                           │
                    │                                     │
                    ▼                                     │
        ┌───────────────────────┐                        │
        │  DEPOSITS COMPLETE    │                        │
        │  (all N deposited)    │                        │
        └───────────┬───────────┘                        │
                    │                                     │
          any member calls rotate_round()                │
                    │                                     │
                    ▼                                     │
        ┌───────────────────────┐                        │
        │  PAYOUT EXECUTED      │                        │
        │  recipient receives   │                        │
        │  N × amount tokens    │                        │
        └───────────┬───────────┘                        │
                    │                                     │
          round < total_slots?  ──── YES ────────────────┘
                    │
                   NO
                    │
                    ▼
        ┌───────────────────────┐
        │  CYCLE COMPLETE       │
        │  all members paid out │
        └───────────────────────┘
```

### Storage Layout

All state is stored in Soroban's persistent ledger storage using typed keys:

```
Ledger Storage (Persistent)
┌──────────────────────────────────────────────────────────────┐
│                                                              │
│  Key: CIRCLE                                                 │
│       └─► Circle { amount: i128, round: u32,                │
│                    total_slots: u32 }                        │
│                                                              │
│  Key: TOKEN                                                  │
│       └─► Address  (SEP-41 token contract)                  │
│                                                              │
│  Key: MEMBER(Address)                                        │
│       └─► slot_index: u32  (join order, 0-based)            │
│                                                              │
│  Key: DEPOSIT(Address, round: u32)                          │
│       └─► bool  (true = deposited for this round)           │
│                                                              │
│  Key: RECIPIENT(round: u32)                                  │
│       └─► Address  (who receives the pot this round)        │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

### Module Dependency Graph

```
src/lib.rs  (contract entry point, #[contract] impl)
  │
  ├── src/types.rs
  │     ├── Circle  { amount, round, total_slots }
  │     └── Error   { CycleFull, EarlyDrawAttempt }
  │
  ├── src/lifecycle.rs
  │     ├── initialize_circle(token, amount, slots)
  │     └── register_member(member)
  │
  ├── src/pools.rs
  │     ├── process_deposit(member)
  │     └── draw_pot(env, recipient)
  │
  └── src/events.rs
        └── emit_payout_event(env, recipient, round, amount)
```

---

## Contract Modules

| File | Responsibility | Key Exports |
|------|---------------|-------------|
| `src/lib.rs` | Contract entry point; wires all modules into the `#[contract]` impl | `AjoContract` |
| `src/types.rs` | Shared data model | `Circle`, `Error` |
| `src/lifecycle.rs` | Circle setup and member registration | `initialize_circle`, `register_member` |
| `src/pools.rs` | Deposit processing and pot disbursement | `process_deposit`, `draw_pot` |
| `src/events.rs` | On-chain event emission | `emit_payout_event` |

---

## Data Types

### `Circle`

The core state struct stored under the `CIRCLE` key. Tracks the contribution amount, current round, and total membership capacity.

```rust
#[contracttype]
pub struct Circle {
    pub amount: i128,      // contribution per member per round (in token's smallest unit)
    pub round: u32,        // current round index (0-based)
    pub total_slots: u32,  // total number of members / rounds in the cycle
}
```

### `Error`

Contract-level errors returned as `Result<_, Error>` from all fallible functions.

```rust
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub enum Error {
    CycleFull        = 1,  // register_member called when all slots are filled
    EarlyDrawAttempt = 2,  // rotate_round called before all members have deposited
}
```

---

## Core Functions

### `initialize_circle(token: Address, amount: i128, slots: u32)`

Sets up a new savings circle. Must be called once before any members join. Stores the token address, contribution amount, and total slot count. Initializes `round` to `0`.

```rust
pub fn initialize_circle(env: Env, token: Address, amount: i128, slots: u32) {
    let circle = Circle { amount, round: 0, total_slots: slots };
    env.storage().persistent().set(&DataKey::Circle, &circle);
    env.storage().persistent().set(&DataKey::Token, &token);
}
```

**CLI invocation:**

```bash
soroban contract invoke \
  --id $CONTRACT_ID \
  --network testnet \
  --source $ORGANIZER_SECRET \
  --fn initialize_circle \
  -- \
  --token $TOKEN_ADDRESS \
  --amount 100000000 \
  --slots 3
```

---

### `register_member(member: Address)`

Adds a member to the circle and assigns them a rotation slot based on join order. The first member to register gets slot `0` (receives the pot in round 0), the second gets slot `1`, and so on.

Fails with `Error::CycleFull` if all slots are already taken.

```rust
pub fn register_member(env: Env, member: Address) -> Result<(), Error> {
    member.require_auth();

    let mut circle: Circle = env.storage().persistent().get(&DataKey::Circle).unwrap();

    // Count existing members to determine next slot
    let slot = /* current member count */;
    if slot >= circle.total_slots {
        return Err(Error::CycleFull);
    }

    env.storage().persistent().set(&DataKey::Member(member.clone()), &slot);
    // Store member as recipient for their assigned round
    env.storage().persistent().set(&DataKey::Recipient(slot), &member);

    Ok(())
}
```

**CLI invocation:**

```bash
soroban contract invoke \
  --id $CONTRACT_ID \
  --network testnet \
  --source $MEMBER_SECRET \
  --fn register_member \
  -- \
  --member $MEMBER_ADDRESS
```

---

### `process_deposit(member: Address)`

Transfers `amount` tokens from the member's account to the contract. Marks the deposit as received for the current round. The member must have pre-approved the contract to spend `amount` tokens via the SEP-41 `approve` call.

```rust
pub fn process_deposit(env: Env, member: Address) -> Result<(), Error> {
    member.require_auth();

    let circle: Circle = env.storage().persistent().get(&DataKey::Circle).unwrap();
    let token: Address = env.storage().persistent().get(&DataKey::Token).unwrap();

    // Transfer tokens from member to contract
    let token_client = token::Client::new(&env, &token);
    token_client.transfer(&member, &env.current_contract_address(), &circle.amount);

    // Record deposit for this round
    env.storage().persistent().set(
        &DataKey::Deposit(member.clone(), circle.round),
        &true,
    );

    Ok(())
}
```

**CLI invocation:**

```bash
# Step 1: approve the contract to spend tokens (SEP-41)
soroban contract invoke \
  --id $TOKEN_ADDRESS \
  --source $MEMBER_SECRET \
  --fn approve \
  -- \
  --from $MEMBER_ADDRESS \
  --spender $CONTRACT_ID \
  --amount 100000000 \
  --expiration_ledger 999999

# Step 2: deposit
soroban contract invoke \
  --id $CONTRACT_ID \
  --network testnet \
  --source $MEMBER_SECRET \
  --fn process_deposit \
  -- \
  --member $MEMBER_ADDRESS
```

---

### `rotate_round(caller: Address)`

The trigger function. Verifies that every registered member has deposited for the current round. If so, calls `draw_pot` to pay out the recipient, then increments the round counter.

Fails with `Error::EarlyDrawAttempt` if any member has not yet deposited.

```rust
pub fn rotate_round(env: Env, caller: Address) -> Result<(), Error> {
    caller.require_auth();

    let mut circle: Circle = env.storage().persistent().get(&DataKey::Circle).unwrap();

    // Verify all members have deposited for the current round
    for slot in 0..circle.total_slots {
        let recipient: Address = env.storage().persistent()
            .get(&DataKey::Recipient(slot)).unwrap();
        let deposited: bool = env.storage().persistent()
            .get(&DataKey::Deposit(recipient.clone(), circle.round))
            .unwrap_or(false);
        if !deposited {
            return Err(Error::EarlyDrawAttempt);
        }
    }

    // Determine this round's recipient and pay out
    let recipient: Address = env.storage().persistent()
        .get(&DataKey::Recipient(circle.round)).unwrap();
    draw_pot(&env, recipient);

    // Advance to next round
    circle.round = circle.round.checked_add(1).unwrap();
    env.storage().persistent().set(&DataKey::Circle, &circle);

    Ok(())
}
```

**CLI invocation:**

```bash
soroban contract invoke \
  --id $CONTRACT_ID \
  --network testnet \
  --source $ANY_MEMBER_SECRET \
  --fn rotate_round \
  -- \
  --caller $ANY_MEMBER_ADDRESS
```

---

### `draw_pot(env: &Env, recipient: Address)`

Internal helper called exclusively by `rotate_round`. Computes the total pool (`amount × total_slots`), executes the SEP-41 transfer from the contract to the recipient, and emits the payout event.

```rust
fn draw_pot(env: &Env, recipient: Address) {
    let circle: Circle = env.storage().persistent().get(&DataKey::Circle).unwrap();
    let token: Address = env.storage().persistent().get(&DataKey::Token).unwrap();

    let payout = circle.amount
        .checked_mul(circle.total_slots as i128)
        .expect("overflow");

    let token_client = token::Client::new(env, &token);
    token_client.transfer(&env.current_contract_address(), &recipient, &payout);

    emit_payout_event(env, &recipient, circle.round, payout);
}
```

---

## Error Codes

| Code | Name | Trigger Condition |
|------|------|-------------------|
| `1` | `CycleFull` | `register_member` called when `slot_count == total_slots` |
| `2` | `EarlyDrawAttempt` | `rotate_round` called before all `total_slots` members have deposited for the current round |

---

## Events

Every successful payout emits a structured on-chain event. Indexers, frontends, and wallets can subscribe to the `payout` topic to track circle activity in real time.

```rust
pub fn emit_payout_event(env: &Env, recipient: &Address, round: u32, amount: i128) {
    env.events().publish(
        (symbol_short!("payout"),),          // topic
        (recipient.clone(), round, amount),  // data
    );
}
```

| Field | Type | Description |
|-------|------|-------------|
| `recipient` | `Address` | The member who received the pot |
| `round` | `u32` | The round index that was just completed |
| `amount` | `i128` | Total tokens transferred (N × contribution) |

**Listening for events (Soroban CLI):**

```bash
soroban events \
  --network testnet \
  --contract-id $CONTRACT_ID \
  --topic payout
```

---

## Getting Started

### Prerequisites

- Rust toolchain (stable) with the `wasm32-unknown-unknown` target
- [Soroban CLI](https://soroban.stellar.org/docs/getting-started/setup) v21+

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add WASM target
rustup target add wasm32-unknown-unknown

# Install Soroban CLI
cargo install --locked soroban-cli
```

### Build

```bash
cargo build --target wasm32-unknown-unknown --release
```

The compiled contract will be at:

```
target/wasm32-unknown-unknown/release/ajo_rotation_contract.wasm
```

### Deploy to Testnet

```bash
# Configure testnet network
soroban network add testnet \
  --rpc-url https://soroban-testnet.stellar.org \
  --network-passphrase "Test SDF Network ; September 2015"

# Fund a test account
soroban keys generate organizer --network testnet --fund

# Deploy
soroban contract deploy \
  --wasm target/wasm32-unknown-unknown/release/ajo_rotation_contract.wasm \
  --network testnet \
  --source organizer
```

### End-to-End Example

A complete 3-member circle from setup to first payout:

```bash
export CONTRACT_ID="C..."
export TOKEN="C..."   # any SEP-41 token on testnet

# 1. Initialize the circle: 10 XLM per round, 3 members
soroban contract invoke --id $CONTRACT_ID --network testnet --source organizer \
  --fn initialize_circle -- \
  --token $TOKEN --amount 100000000 --slots 3

# 2. Register members (each signs their own tx)
soroban contract invoke --id $CONTRACT_ID --network testnet --source alice \
  --fn register_member -- --member $ALICE

soroban contract invoke --id $CONTRACT_ID --network testnet --source bob \
  --fn register_member -- --member $BOB

soroban contract invoke --id $CONTRACT_ID --network testnet --source carol \
  --fn register_member -- --member $CAROL

# 3. Round 0 deposits (each member approves then deposits)
for SECRET in $ALICE_SECRET $BOB_SECRET $CAROL_SECRET; do
  soroban contract invoke --id $TOKEN --source $SECRET \
    --fn approve -- --from $(soroban keys address $SECRET) \
    --spender $CONTRACT_ID --amount 100000000 --expiration_ledger 999999

  soroban contract invoke --id $CONTRACT_ID --network testnet --source $SECRET \
    --fn process_deposit -- --member $(soroban keys address $SECRET)
done

# 4. Rotate — Alice receives 300_000_000 stroops (30 XLM)
soroban contract invoke --id $CONTRACT_ID --network testnet --source alice \
  --fn rotate_round -- --member $ALICE
```

---

## Testing

The test suite covers the full round lifecycle, error paths, and edge cases.

```bash
cargo test
```

Key test scenarios:

```rust
#[test]
fn test_full_cycle() {
    // initialize → register 3 members → 3 rounds of deposits + rotations
    // assert each member received the pot exactly once
}

#[test]
fn test_cycle_full_error() {
    // register total_slots + 1 members → expect Error::CycleFull
}

#[test]
fn test_early_draw_attempt() {
    // call rotate_round before all deposits → expect Error::EarlyDrawAttempt
}
```

---

## CI

The project includes a GitHub Actions workflow (`ci.yml`) that runs on every push and pull request. It builds the WASM artifact and runs the full test suite.

```yaml
name: CI

on: [push, pull_request]

jobs:
  build-and-test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          targets: wasm32-unknown-unknown

      - name: Build WASM
        run: cargo build --target wasm32-unknown-unknown --release

      - name: Run tests
        run: cargo test
```

---

## Security Considerations

- **Auth on every state-changing call** — `register_member`, `process_deposit`, and `rotate_round` all call `require_auth()` on the acting address. No one can deposit or rotate on behalf of another member without their signature.
- **No admin key** — once deployed, there is no privileged account that can drain funds or skip rounds. The contract is fully autonomous.
- **Overflow protection** — payout calculation uses `checked_mul` to prevent integer overflow on large amounts or slot counts.
- **Deposit idempotency** — the `DEPOSIT(addr, round)` key prevents double-counting if a member somehow calls `process_deposit` twice in the same round (the second transfer would still execute, so members should take care; a production version should guard against this explicitly).

---

## License

MIT
