# Futurity Protocol – Solana Smart Contract

## Overview

**Futurity** is a secure, time-locked escrow protocol built on Solana using Anchor. It enables users to lock SOL in a program-derived address (PDA) escrow account, with dual withdrawal authority (either the sender or recipient can withdraw after the unlock time). The protocol supports optional account closure after a grace period, enforces a minimum deposit (rent-exempt or 0.001 SOL, whichever is greater), and provides robust error handling for all escrow scenarios.

**Key Features:**
- **Time-Locked Escrow:** Funds are locked until a specified unlock time (minimum 60 seconds, maximum 100 years).
- **Dual Withdrawal Authority:** Either sender or recipient can withdraw after unlock.
- **PDA-Based Security:** Escrow accounts are program-derived and unique per (sender, recipient, amount, unlock_time).
- **Graceful Closure:** Anyone can close expired deposits after a 1-year grace period, forwarding remaining rent to a user-chosen account.
- **Minimum Deposit:** Enforces a minimum deposit (rent-exempt or 0.001 SOL, whichever is greater).
- **Extensive Error Handling:** Covers invalid unlock times, unauthorized withdrawals, double-withdrawal, and more.
- **Security.txt:** Embedded for responsible disclosure.
- **Event Emission:** Emits events for creation, withdrawal, and closure for easy off-chain indexing.
- **Test Suite:** Comprehensive Anchor test suite covers happy path and edge cases (see below).

## Directory Structure

```
solana/
├── Anchor.toml           # Anchor configuration
├── README.md             # This file
├── program/
│   ├── Cargo.toml        # Rust/Anchor program config
│   ├── src/
│   │   └── lib.rs        # Main program logic
│   └── tests/
│       └── escrow.ts     # Anchor test suite (TypeScript)
└── ...                   # Other build/test artifacts
```

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install)
- [Solana CLI](https://docs.solana.com/cli/install-solana-cli-tools)
- [Anchor CLI](https://book.anchor-lang.com/chapter_1/installation.html)
- [Node.js & Yarn/NPM](https://nodejs.org/) (for running TypeScript tests)

Ensure you have a local validator or devnet access for testing/deployment.

## Building & Compiling

From the `solana` directory, run:

```bash
anchor build
```

This will compile the program and output artifacts to `program/target/deploy`.

## Testing

To run the program’s tests (including edge cases and fuzzing):

```bash
anchor test
```

This will spin up a local validator, build, deploy, and execute all tests in `program/tests/escrow.ts`.

### Test Coverage
- **Happy path:** create, withdraw, close
- **Edge cases:**
  - Minimum deposit (rent-exempt and 0.001 SOL)
  - Minimum/maximum lock duration
  - Early/late withdrawal attempts
  - Double withdrawal prevention
  - Excessive unlock time
  - Closure after grace period
  - Forced lamport injection (see test for details)

## Deploying

Before you deploy, copy the asset to the top level directory target subdirectory.

```bash
cp program/target/deploy/futurity.so target/deploy/.
```

To deploy to a cluster (e.g., devnet):

```bash
anchor deploy --provider.cluster devnet
```

Ensure your wallet is funded and configured correctly (see `~/.config/solana/cli/config.yml`).

## Validating Deployment

After deployment, you can validate the program:

1. **Check program ID:** Confirm the deployed program ID matches the one in `declare_id!` in `lib.rs`.
2. **Interact with the program:** Use Anchor scripts, Solana CLI, or a frontend to create deposits, withdraw, and close expired escrows.
3. **Logs & Events:** Review logs using `solana logs` or Anchor test output for emitted events (e.g., `EscrowCreatedEvent`, `EscrowWithdrawnEvent`, `EscrowClosedEvent`).

## Additional Resources

- [Anchor Docs](https://book.anchor-lang.com/)
- [Solana Docs](https://docs.solana.com/)

---

Feel free to update this README with more usage examples, API details, or test scenarios as the protocol evolves.
