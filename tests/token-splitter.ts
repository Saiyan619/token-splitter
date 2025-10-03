import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { TokenSplitter } from "../target/types/token_splitter";
import { 
  createMint, 
  getOrCreateAssociatedTokenAccount, 
  mintTo,
  getAccount,
  TOKEN_PROGRAM_ID
} from "@solana/spl-token";
import { PublicKey, Keypair, SystemProgram } from "@solana/web3.js";
import { assert } from "chai";

describe("token_splitter", () => {
  // Configure the client to use the local cluster
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.TokenSplitter as Program<TokenSplitter>;
  const connection = provider.connection;
  const payer = provider.wallet as anchor.Wallet;

  // Verify we're on localhost
  console.log("RPC Endpoint:", connection.rpcEndpoint);
  if (!connection.rpcEndpoint.includes("localhost") && !connection.rpcEndpoint.includes("127.0.0.1")) {
    throw new Error(`Tests must run on localhost! Current endpoint: ${connection.rpcEndpoint}`);
  }

  let mint: PublicKey;
  let userTokenAccount: PublicKey;
  let vaultInfo: PublicKey;
  let vaultTokenAccount: PublicKey;
  let vaultInfoBump: number;
  let vaultTokenBump: number;

  // Target accounts for testing share_funds
  let targetAccounts: PublicKey[] = [];
  const NUM_TARGETS = 3;

  before(async () => {
    // Create a new mint
    mint = await createMint(
      connection,
      payer.payer,
      payer.publicKey,
      null,
      9 // 9 decimals
    );

    console.log("Mint created:", mint.toBase58());

    // Create user token account
    const userTokenAccountInfo = await getOrCreateAssociatedTokenAccount(
      connection,
      payer.payer,
      mint,
      payer.publicKey
    );
    userTokenAccount = userTokenAccountInfo.address;

    // Mint tokens to user account
    await mintTo(
      connection,
      payer.payer,
      mint,
      userTokenAccount,
      payer.publicKey,
      1000000000000 // 1000 tokens with 9 decimals
    );

    console.log("User token account created and funded:", userTokenAccount.toBase58());

    // Derive PDA addresses
    [vaultInfo, vaultInfoBump] = PublicKey.findProgramAddressSync(
      [Buffer.from("vault_info"), payer.publicKey.toBuffer(), mint.toBuffer()],
      program.programId
    );

    [vaultTokenAccount, vaultTokenBump] = PublicKey.findProgramAddressSync(
      [Buffer.from("token_vault"), payer.publicKey.toBuffer(), mint.toBuffer()],
      program.programId
    );

    console.log("Vault Info PDA:", vaultInfo.toBase58());
    console.log("Vault Token Account PDA:", vaultTokenAccount.toBase58());

    // Create target token accounts for share_funds testing
    for (let i = 0; i < NUM_TARGETS; i++) {
      const targetKeypair = Keypair.generate();
      const targetTokenAccount = await getOrCreateAssociatedTokenAccount(
        connection,
        payer.payer,
        mint,
        targetKeypair.publicKey
      );
      targetAccounts.push(targetTokenAccount.address);
      console.log(`Target ${i + 1} token account:`, targetTokenAccount.address.toBase58());
    }
  });

  it("Initializes the vault", async () => {
    const tx = await program.methods
      .initializeVault()
      .accounts({
        vaultInfo,
        vaultTokenAcc: vaultTokenAccount,
        signer: payer.publicKey,
        mint,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    console.log("Initialize vault transaction:", tx);

    // Fetch and verify vault info
    const vaultInfoAccount = await program.account.vaultInfo.fetch(vaultInfo);
    assert.equal(vaultInfoAccount.owner.toBase58(), payer.publicKey.toBase58());
    assert.equal(vaultInfoAccount.mint.toBase58(), mint.toBase58());
    assert.equal(vaultInfoAccount.amount.toString(), "0");
    assert.equal(vaultInfoAccount.vaultInfoBump, vaultInfoBump);
    assert.equal(vaultInfoAccount.vaultTokenBump, vaultTokenBump);
    assert.isTrue(vaultInfoAccount.createdAt.toNumber() > 0);

    console.log("Vault initialized successfully");
  });

  it("Deposits tokens into the vault", async () => {
    const depositAmount = new anchor.BN(500000000000); // 500 tokens

    const userBalanceBefore = await getAccount(connection, userTokenAccount);
    
    const tx = await program.methods
      .depositVault(depositAmount)
      .accounts({
        vaultInfo,
        vaultTokenAcc: vaultTokenAccount,
        userTokenAcc: userTokenAccount,
        signer: payer.publicKey,
        mint,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    console.log("Deposit transaction:", tx);

    // Verify balances
    const userBalanceAfter = await getAccount(connection, userTokenAccount);
    const vaultBalance = await getAccount(connection, vaultTokenAccount);
    const vaultInfoAccount = await program.account.vaultInfo.fetch(vaultInfo);

    assert.equal(
      userBalanceBefore.amount - userBalanceAfter.amount,
      depositAmount.toNumber()
    );
    assert.equal(vaultBalance.amount.toString(), depositAmount.toString());
    assert.equal(vaultInfoAccount.amount.toString(), depositAmount.toString());

    console.log("Deposit successful. Vault balance:", vaultBalance.amount.toString());
  });

  it("Fails to deposit zero amount", async () => {
    try {
      await program.methods
        .depositVault(new anchor.BN(0))
        .accounts({
          vaultInfo,
          vaultTokenAcc: vaultTokenAccount,
          userTokenAcc: userTokenAccount,
          signer: payer.publicKey,
          mint,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .rpc();
      assert.fail("Should have thrown an error");
    } catch (error) {
      assert.include(error.message, "you cant deposit 0");
      console.log("Zero deposit correctly rejected");
    }
  });

  it("Shares funds equally among targets", async () => {
    const vaultBalanceBefore = await getAccount(connection, vaultTokenAccount);
    const targetBalancesBefore = await Promise.all(
      targetAccounts.map(acc => getAccount(connection, acc))
    );

    const tx = await program.methods
      .shareFunds()
      .accounts({
        vaultInfo,
        vaultTokenAcc: vaultTokenAccount,
        signer: payer.publicKey,
        userTokenAcc: userTokenAccount,
        mint,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .remainingAccounts(
        targetAccounts.map(acc => ({
          pubkey: acc,
          isWritable: true,
          isSigner: false,
        }))
      )
      .rpc();

    console.log("Share funds transaction:", tx);

    // Verify distributions
    const vaultBalanceAfter = await getAccount(connection, vaultTokenAccount);
    const targetBalancesAfter = await Promise.all(
      targetAccounts.map(acc => getAccount(connection, acc))
    );
    const vaultInfoAccount = await program.account.vaultInfo.fetch(vaultInfo);

    const expectedSplitAmount = vaultBalanceBefore.amount / BigInt(NUM_TARGETS);
    const expectedRemainder = vaultBalanceBefore.amount % BigInt(NUM_TARGETS);

    // Check each target received the correct amount
    for (let i = 0; i < NUM_TARGETS; i++) {
      const received = targetBalancesAfter[i].amount - targetBalancesBefore[i].amount;
      assert.equal(received, expectedSplitAmount);
    }

    // Vault should be empty (or have minimal dust if any)
    assert.equal(vaultBalanceAfter.amount, 0n);
    assert.equal(vaultInfoAccount.amount.toString(), "0");

    console.log("Funds shared successfully");
    console.log("Each target received:", expectedSplitAmount.toString());
    console.log("Remainder returned to user:", expectedRemainder.toString());
  });

  it("Deposits more tokens for withdrawal test", async () => {
    const depositAmount = new anchor.BN(300000000000); // 300 tokens

    await program.methods
      .depositVault(depositAmount)
      .accounts({
        vaultInfo,
        vaultTokenAcc: vaultTokenAccount,
        userTokenAcc: userTokenAccount,
        signer: payer.publicKey,
        mint,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    console.log("Deposited for withdrawal test");
  });

  it("Withdraws all tokens from vault", async () => {
    const userBalanceBefore = await getAccount(connection, userTokenAccount);
    const vaultBalanceBefore = await getAccount(connection, vaultTokenAccount);

    const tx = await program.methods
      .withdraw()
      .accounts({
        vaultInfo,
        vaultTokenAcc: vaultTokenAccount,
        userTokenAcc: userTokenAccount,
        signer: payer.publicKey,
        mint,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    console.log("Withdraw transaction:", tx);

    // Verify balances
    const userBalanceAfter = await getAccount(connection, userTokenAccount);
    const vaultBalanceAfter = await getAccount(connection, vaultTokenAccount);
    const vaultInfoAccount = await program.account.vaultInfo.fetch(vaultInfo);

    assert.equal(
      userBalanceAfter.amount - userBalanceBefore.amount,
      vaultBalanceBefore.amount
    );
    assert.equal(vaultBalanceAfter.amount, 0n);
    assert.equal(vaultInfoAccount.amount.toString(), "0");

    console.log("Withdrawal successful. User received:", vaultBalanceBefore.amount.toString());
  });

  it("Fails to withdraw from empty vault", async () => {
    try {
      await program.methods
        .withdraw()
        .accounts({
          vaultInfo,
          vaultTokenAcc: vaultTokenAccount,
          userTokenAcc: userTokenAccount,
          signer: payer.publicKey,
          mint,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .rpc();
      assert.fail("Should have thrown an error");
    } catch (error) {
      assert.include(error.message, "Vault is empty");
      console.log("Empty vault withdrawal correctly rejected");
    }
  });

  it("Closes the vault", async () => {
    const userSolBalanceBefore = await connection.getBalance(payer.publicKey);

    const tx = await program.methods
      .closeVault()
      .accounts({
        vaultInfo,
        vaultTokenAcc: vaultTokenAccount,
        signer: payer.publicKey,
        userTokenAcc: userTokenAccount,
        mint,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    console.log("Close vault transaction:", tx);

    // Verify vault is closed
    try {
      await program.account.vaultInfo.fetch(vaultInfo);
      assert.fail("Vault should be closed");
    } catch (error) {
      assert.include(error.message, "Account does not exist");
    }

    // Verify SOL was reclaimed
    const userSolBalanceAfter = await connection.getBalance(payer.publicKey);
    assert.isTrue(userSolBalanceAfter > userSolBalanceBefore);

    console.log("Vault closed successfully");
  });

  it("Fails to close vault with non-zero balance", async () => {
    // Reinitialize vault (after it was closed in previous test)
    await program.methods
      .initializeVault()
      .accounts({
        vaultInfo,
        vaultTokenAcc: vaultTokenAccount,
        signer: payer.publicKey,
        mint,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    console.log("Vault reinitialized for close test");

    // Deposit some tokens
    await program.methods
      .depositVault(new anchor.BN(100000000000))
      .accounts({
        vaultInfo,
        vaultTokenAcc: vaultTokenAccount,
        userTokenAcc: userTokenAccount,
        signer: payer.publicKey,
        mint,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    console.log("Deposited tokens for close test");

    // Try to close
    try {
      await program.methods
        .closeVault()
        .accounts({
          vaultInfo,
          vaultTokenAcc: vaultTokenAccount,
          signer: payer.publicKey,
          userTokenAcc: userTokenAccount,
          mint,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .rpc();
      assert.fail("Should have thrown an error");
    } catch (error) {
      assert.include(error.message, "Vault is not empty");
      console.log("Non-empty vault close correctly rejected");
    }

    // Clean up - withdraw and close for next tests
    await program.methods
      .withdraw()
      .accounts({
        vaultInfo,
        vaultTokenAcc: vaultTokenAccount,
        userTokenAcc: userTokenAccount,
        signer: payer.publicKey,
        mint,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    // Close the vault so next tests start fresh
    await program.methods
      .closeVault()
      .accounts({
        vaultInfo,
        vaultTokenAcc: vaultTokenAccount,
        signer: payer.publicKey,
        userTokenAcc: userTokenAccount,
        mint,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    console.log("Vault cleaned up and closed");
  });

  it("Fails to share funds with no targets", async () => {
    // Reinitialize vault if needed
    try {
      await program.account.vaultInfo.fetch(vaultInfo);
    } catch {
      // Vault doesn't exist, reinitialize
      await program.methods
        .initializeVault()
        .accounts({
          vaultInfo,
          vaultTokenAcc: vaultTokenAccount,
          signer: payer.publicKey,
          mint,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .rpc();
    }

    // Ensure vault has some tokens
    await program.methods
      .depositVault(new anchor.BN(100000000000))
      .accounts({
        vaultInfo,
        vaultTokenAcc: vaultTokenAccount,
        userTokenAcc: userTokenAccount,
        signer: payer.publicKey,
        mint,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    try {
      await program.methods
        .shareFunds()
        .accounts({
          vaultInfo,
          vaultTokenAcc: vaultTokenAccount,
          signer: payer.publicKey,
          userTokenAcc: userTokenAccount,
          mint,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .remainingAccounts([]) // No targets
        .rpc();
      assert.fail("Should have thrown an error");
    } catch (error) {
      assert.include(error.message, "No target addresses provided");
      console.log("Share funds with no targets correctly rejected");
    }
  });

  it("Fails to share funds with more than 20 targets", async () => {
    // Ensure vault has tokens
    const vaultBalance = await getAccount(connection, vaultTokenAccount);
    if (vaultBalance.amount === 0n) {
      await program.methods
        .depositVault(new anchor.BN(100000000000))
        .accounts({
          vaultInfo,
          vaultTokenAcc: vaultTokenAccount,
          userTokenAcc: userTokenAccount,
          signer: payer.publicKey,
          mint,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .rpc();
    }

    // Create 21 target accounts
    const manyTargets = [];
    for (let i = 0; i < 21; i++) {
      const targetKeypair = Keypair.generate();
      const targetTokenAccount = await getOrCreateAssociatedTokenAccount(
        connection,
        payer.payer,
        mint,
        targetKeypair.publicKey
      );
      manyTargets.push(targetTokenAccount.address);
    }

    try {
      await program.methods
        .shareFunds()
        .accounts({
          vaultInfo,
          vaultTokenAcc: vaultTokenAccount,
          signer: payer.publicKey,
          userTokenAcc: userTokenAccount,
          mint,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .remainingAccounts(
          manyTargets.map(acc => ({
            pubkey: acc,
            isWritable: true,
            isSigner: false,
          }))
        )
        .rpc();
      assert.fail("Should have thrown an error");
    } catch (error) {
      assert.include(error.message, "Maximum 20 targets allowed");
      console.log("Share funds with >20 targets correctly rejected");
    }
  });
});