import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Futurity } from "../target/types/futurity";
import { assert } from "chai";

const MIN_DEPOSIT = new anchor.BN(1_000_000); // 0.001 SOL
const MIN_LOCK = 60; // seconds
const MAX_LOCK = 100 * 365 * 24 * 60 * 60; // 100 years
const CLOSE_GRACE = 365 * 24 * 60 * 60; // 1 year

describe("futurity escrow", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.Futurity as Program<Futurity>;

  let sender = provider.wallet;
  let recipient = anchor.web3.Keypair.generate();
  let depositPda: anchor.web3.PublicKey;
  let depositBump: number;
  let depositAmount = MIN_DEPOSIT.addn(10000);
  let unlockTime: number;

  it("Creates a time-locked deposit (happy path)", async () => {
    unlockTime = Math.floor(Date.now() / 1000) + MIN_LOCK + 5;
    [depositPda, depositBump] = anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from("futurity_escrow"),
        sender.publicKey.toBuffer(),
        recipient.publicKey.toBuffer(),
        depositAmount.toArrayLike(Buffer, "le", 8),
        new anchor.BN(unlockTime).toArrayLike(Buffer, "le", 8),
      ],
      program.programId
    );
    await program.methods
      .createTimeLockDeposit(depositAmount, new anchor.BN(unlockTime))
      .accounts({
        sender: sender.publicKey,
        recipient: recipient.publicKey,
        deposit: depositPda,
        systemProgram: anchor.web3.SystemProgram.programId,
      })
      .signers([])
      .rpc();
    const deposit = await program.account.timeLockDeposit.fetch(depositPda);
    assert.ok(deposit.amount.eq(depositAmount));
    assert.ok(deposit.sender.equals(sender.publicKey));
    assert.ok(deposit.recipient.equals(recipient.publicKey));
    assert.ok(!deposit.isWithdrawn);
  });

  it("Fails to withdraw before unlock time", async () => {
    try {
      await program.methods
        .withdrawDeposit()
        .accounts({
          withdrawer: recipient.publicKey,
          deposit: depositPda,
        })
        .signers([recipient])
        .rpc();
      assert.fail("Should not allow early withdrawal");
    } catch (e) {
      assert.match(e.message, /StillLocked/);
    }
  });

  it("Withdraws after unlock time (happy path)", async () => {
    await new Promise((r) => setTimeout(r, (unlockTime - Math.floor(Date.now() / 1000) + 1) * 1000));
    await program.methods
      .withdrawDeposit()
      .accounts({
        withdrawer: recipient.publicKey,
        deposit: depositPda,
      })
      .signers([recipient])
      .rpc();
    const deposit = await program.account.timeLockDeposit.fetch(depositPda);
    assert.ok(deposit.isWithdrawn);
    assert.ok(deposit.withdrawnBy.equals(recipient.publicKey));
  });

  it("Fails to withdraw twice", async () => {
    try {
      await program.methods
        .withdrawDeposit()
        .accounts({
          withdrawer: recipient.publicKey,
          deposit: depositPda,
        })
        .signers([recipient])
        .rpc();
      assert.fail("Should not allow double withdrawal");
    } catch (e) {
      assert.match(e.message, /AlreadyWithdrawn/);
    }
  });

  it("Fails to create deposit below min/rent", async () => {
    let tooSmall = new anchor.BN(1);
    let [pda] = anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from("futurity_escrow"),
        sender.publicKey.toBuffer(),
        recipient.publicKey.toBuffer(),
        tooSmall.toArrayLike(Buffer, "le", 8),
        new anchor.BN(unlockTime + 100).toArrayLike(Buffer, "le", 8),
      ],
      program.programId
    );
    try {
      await program.methods
        .createTimeLockDeposit(tooSmall, new anchor.BN(unlockTime + 100))
        .accounts({
          sender: sender.publicKey,
          recipient: recipient.publicKey,
          deposit: pda,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .signers([])
        .rpc();
      assert.fail("Should not allow deposit below min/rent");
    } catch (e) {
      assert.match(e.message, /InsufficientAmount/);
    }
  });

  it("Fails to create deposit with unlock_time too far in future", async () => {
    let farFuture = Math.floor(Date.now() / 1000) + MAX_LOCK + 1000;
    let [pda] = anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from("futurity_escrow"),
        sender.publicKey.toBuffer(),
        recipient.publicKey.toBuffer(),
        depositAmount.toArrayLike(Buffer, "le", 8),
        new anchor.BN(farFuture).toArrayLike(Buffer, "le", 8),
      ],
      program.programId
    );
    try {
      await program.methods
        .createTimeLockDeposit(depositAmount, new anchor.BN(farFuture))
        .accounts({
          sender: sender.publicKey,
          recipient: recipient.publicKey,
          deposit: pda,
          systemProgram: anchor.web3.SystemProgram.programId,
        })
        .signers([])
        .rpc();
      assert.fail("Should not allow excessive unlock_time");
    } catch (e) {
      assert.match(e.message, /ExcessiveUnlockTime/);
    }
  });

  it("Allows anyone to close after grace period", async () => {
    // Simulate grace period expiry
    const deposit = await program.account.timeLockDeposit.fetch(depositPda);
    const closeTime = deposit.unlockTime.toNumber() + CLOSE_GRACE + 1;
    const now = Math.floor(Date.now() / 1000);
    if (closeTime > now) {
      await new Promise((r) => setTimeout(r, (closeTime - now) * 1000));
    }
    await program.methods
      .closeExpiredDeposit()
      .accounts({
        closer: sender.publicKey,
        rentReceiver: sender.publicKey,
        deposit: depositPda,
      })
      .signers([])
      .rpc();
    // Should not throw
  });

  // Add more edge/fuzz tests as needed
});
