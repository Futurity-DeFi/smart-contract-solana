# Futurity Protocol – Solana Smart Contract

## Overview

**Futurity** is a secure, time-locked escrow protocol built on Solana. The program is now implemented using native Solana Rust APIs instead of Anchor. It enables users to lock SOL in a program-derived address (PDA) escrow account, with dual withdrawal authority (either the sender or recipient can withdraw after the unlock time). The protocol supports optional account closure after a grace period, enforces a minimum deposit (rent-exempt or 0.001 SOL, whichever is greater), and provides robust error handling for all escrow scenarios.

**Key Features:**
- **Time-Locked Escrow:** Funds are locked until a specified unlock time (minimum 60 seconds, maximum 100 years).
- **Dual Withdrawal Authority:** Either sender or recipient can withdraw after unlock.
- **PDA-Based Security:** Escrow accounts are program-derived and unique per (sender, recipient, amount, unlock_time).
- **Graceful Closure:** Anyone can close expired deposits after a 1-year grace period, forwarding remaining rent to a user-chosen account.
- **Minimum Deposit:** Enforces a minimum deposit (rent-exempt or 0.001 SOL, whichever is greater).
- **Extensive Error Handling:** Covers invalid unlock times, unauthorized withdrawals, double-withdrawal, and more.
- **Security.txt:** Embedded for responsible disclosure.
- **Event Emission:** Emits log messages for creation, withdrawal, and closure for easy off-chain indexing.
- **Test Suite:** Coming soon – example tests can be written using `solana-program-test`.

## Directory Structure

```
solana/
├── README.md             # This file
├── program/
│   ├── Cargo.toml        # Rust program config
│   └── programs/
│       └── futurity/     # Native Solana program source
└── ...                   # Other build/test artifacts
```

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install)
- [Solana CLI](https://docs.solana.com/cli/install-solana-cli-tools)
*Optional:* [Node.js & Yarn/NPM](https://nodejs.org/) if you wish to write JavaScript tests

Ensure you have a local validator or devnet access for testing/deployment.

## Building & Compiling

From the repository root, run:

```bash
cargo build-bpf
```

This will compile the program and output artifacts to `program/target/deploy`.

## Testing

To run the program’s tests (including edge cases and fuzzing):

```bash
cargo test-bpf
```

This will run any Rust-based tests using `solana-program-test`.

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

To deploy to a cluster (e.g., devnet):

```bash
solana program deploy <PATH_TO_SO_BINARY>
```

Ensure your wallet is funded and configured correctly (see `~/.config/solana/cli/config.yml`).

## Validating Deployment

After deployment, you can validate the program:

1. **Check program ID:** Confirm the deployed program ID matches the one in `declare_id!` in `lib.rs`.
2. **Interact with the program:** Use scripts or the Solana CLI to create deposits, withdraw, and close expired escrows.
3. **Logs & Events:** Review logs using `solana logs` for messages such as `Escrow created` or `Escrow withdrawn`.

## Additional Resources

- [Solana Docs](https://docs.solana.com/)

---

Feel free to update this README with more usage examples, API details, or test scenarios as the protocol evolves.
