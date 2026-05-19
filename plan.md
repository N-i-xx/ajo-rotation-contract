# Vero Ajo Rotation Contract — Wave Program Contribution Plan

The Wave Program works by having maintainers create scoped issues that contributors pick up during sprint cycles. Below is a description of the types of work we post, what each category covers, and what a good contribution looks like.

---

## 1. Bug Fixes

These are the most immediately impactful issues. We post them when we identify incorrect behavior, edge cases that aren't handled, or logic that diverges from the spec.

**Examples of bugs we'd post:**

- `process_deposit` does not guard against a member depositing twice in the same round. The second call succeeds and transfers tokens again without updating any new state, effectively letting a member double-pay and inflate the contract balance. A fix would add a check against the existing `DEPOSIT(addr, round)` key before executing the transfer.
- `rotate_round` does not verify that the caller is a registered member — any address that can sign a transaction can trigger a rotation. The fix is to check for a `MEMBER(caller)` storage entry before proceeding.
- Integer edge case: if `total_slots` is set to `0` during `initialize_circle`, the contract initializes successfully but every subsequent call panics or behaves unexpectedly. A fix adds an input validation guard at initialization time.

Bug fix issues are tagged `bug` and `good-first-issue` when the scope is narrow and self-contained.

---

## 2. New Features

Feature issues expand what the contract can do. We scope them carefully so they don't require a full redesign — contributors should be able to implement them within a single sprint.

**Examples of features we'd post:**

- **Deposit idempotency guard** — extend `process_deposit` to return early (or a distinct error code) if the member has already deposited for the current round, rather than executing a second transfer.
- **Cycle completion event** — emit a dedicated on-chain event when `round == total_slots`, signaling that the full cycle is done. Frontends and indexers can use this to close out the circle UI.
- **Member count query** — add a read-only `get_member_count` function that returns how many slots are currently filled. Useful for UIs that need to show registration progress without reading raw storage.
- **Recipient lookup** — add a `get_recipient(round: u32) -> Address` view function so clients can display the payout schedule without reconstructing it off-chain.

Feature issues are tagged `enhancement`. Larger features that touch multiple modules will include a design note in the issue body.

---

## 3. Documentation

Good documentation is a first-class contribution. We post doc issues when function-level comments are missing, when the README drifts from the actual implementation, or when a concept needs a clearer explanation for new contributors.

**Examples of doc work we'd post:**

- Add `///` doc comments to every public function in `lifecycle.rs`, `pools.rs`, and `events.rs` explaining parameters, return values, and failure conditions.
- Write a `CONTRIBUTING.md` that explains how to set up the local dev environment, run tests, and submit a PR.
- Add inline comments to the `rotate_round` deposit-verification loop explaining why it iterates by slot index rather than by address.
- Expand the README's Security Considerations section with a concrete example of what happens if a member calls `process_deposit` twice and why the current version is vulnerable.

Doc issues are tagged `documentation` and are a great entry point for contributors who are new to Soroban but comfortable with technical writing.

---

## 4. Testing

We post testing issues when coverage is missing for a specific path, when an existing test is brittle, or when we want property-based or fuzz tests added.

**Examples of testing work we'd post:**

- Write a test for the double-deposit scenario: call `process_deposit` twice for the same member in the same round and assert the expected behavior (currently a bug; the test should document the failure until the fix lands).
- Add a test for `rotate_round` called by an unregistered address — assert it returns an auth error or a new `NotAMember` error once that guard is added.
- Write a full 5-member cycle test: initialize with 5 slots, register 5 members, run 5 complete rounds, and assert that each member received the pot exactly once and the contract balance returns to zero.
- Add boundary tests for `initialize_circle` with `slots = 0`, `amount = 0`, and `amount = i128::MAX`.

Test issues are tagged `testing`. Contributors are expected to use the Soroban test environment (`soroban_sdk::testutils`) and follow the existing test structure in `src/lib.rs`.

---

## 5. Refactoring & Code Quality

These issues improve the internal structure of the contract without changing external behavior. We post them when we spot repeated patterns, unclear naming, or opportunities to reduce cognitive load for future contributors.

**Examples of refactor work we'd post:**

- Extract the deposit-verification loop in `rotate_round` into a private helper `all_deposits_received(&env, &circle) -> bool` to make the main function body easier to read.
- Consolidate all `env.storage().persistent().get(...).unwrap()` calls behind typed accessor functions to reduce boilerplate and make missing-key panics easier to trace.
- Rename `DataKey::Recipient` to `DataKey::RoundRecipient` to make the key's scope explicit when reading storage dumps.

Refactor issues are tagged `refactor` and always include a note confirming that no behavior change is expected — the existing test suite must pass without modification.
