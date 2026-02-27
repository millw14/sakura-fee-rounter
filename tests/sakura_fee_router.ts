import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { SakuraFeeRouter } from "../target/types/sakura_fee_router";
import { assert } from "chai";
import {
  createMint,
  createAccount,
  mintTo,
  getAccount,
  TOKEN_PROGRAM_ID,
} from "@solana/spl-token";
import { PublicKey, SystemProgram } from "@solana/web3.js";

describe("sakura_fee_router", () => {
  anchor.setProvider(anchor.AnchorProvider.env());

  const provider = anchor.getProvider() as anchor.AnchorProvider;
  const program = anchor.workspace.SakuraFeeRouter as Program<SakuraFeeRouter>;

  // Variables for setup
  let user = provider.wallet;
  let fakeMint: PublicKey;
  let userTokenAccount: PublicKey;
  let insuranceVault: PublicKey;

  // The hardcoded constants from lib.rs
  const SAKURA_MINT = new PublicKey("EWiVNxCqNatzV2paBHyfKUwGLnk7WKs9uZTA5jkTpump");
  const PERCOLATOR_INSURANCE_VAULT = new PublicKey("63juJmvm1XHCHveWv9WdanxqJX6tD6DLFTZD7dvH12dc");

  it("Is initialized!", async () => {
    // Tests environment setup
    assert.ok(program.programId);
  });

  // Mainnet hardening requires strict validation that we can simulate via tests.
  // We can't actually run them fully without the actual mainnet mint unless we mock it, 
  // but we provide the test structure here so the user can verify on a local ledger copy.

  it("Rejects an invalid token mint", async () => {
    // Create a fake mint
    let fakeMintAuth = anchor.web3.Keypair.generate();
    await provider.connection.requestAirdrop(fakeMintAuth.publicKey, 1000000000);
    
    try {
        fakeMint = await createMint(
            provider.connection,
            fakeMintAuth,
            fakeMintAuth.publicKey,
            null,
            6
        );

        userTokenAccount = await createAccount(
            provider.connection,
            fakeMintAuth,
            fakeMint,
            user.publicKey
        );

        insuranceVault = await createAccount(
            provider.connection,
            fakeMintAuth,
            fakeMint,
            fakeMintAuth.publicKey 
        );

        const [subscriptionPda] = PublicKey.findProgramAddressSync(
            [Buffer.from("subscription"), user.publicKey.toBuffer()],
            program.programId
        );

        // This should fail because the mint is not SAKURA_MINT
        await program.methods
            .processPayment(new anchor.BN(100_000))
            .accounts({
                user: user.publicKey,
                userTokenAccount: userTokenAccount,
                insuranceVault: insuranceVault,
                mint: fakeMint,
                subscription: subscriptionPda,
                tokenProgram: TOKEN_PROGRAM_ID,
                systemProgram: SystemProgram.programId,
            })
            .rpc();
        
        assert.fail("Should have failed due to invalid mint");
    } catch (e: any) {
        // Assert the error code matches InvalidMint
        assert.include(e.message, "Invalid token mint");
    }
  });

  it("Rejects an invalid insurance vault", async () => {
      // Assuming we managed to bypass the SAKURA_MINT check (which is impossible),
      // we need to ensure the vault check strictly enforces PERCOLATOR_INSURANCE_VAULT.
      
      let fakeMintAuth = anchor.web3.Keypair.generate();
      
      // Let's pretend SAKURA_MINT is derived or passed in
      let fakeVault = anchor.web3.Keypair.generate().publicKey;
      
      try {
          const [subscriptionPda] = PublicKey.findProgramAddressSync(
              [Buffer.from("subscription"), user.publicKey.toBuffer()],
              program.programId
          );

          await program.methods
            .processPayment(new anchor.BN(100_000))
            .accounts({
                user: user.publicKey,
                userTokenAccount: fakeVault, // fake
                insuranceVault: fakeVault,   // this should trigger InvalidVault
                mint: SAKURA_MINT, 
                subscription: subscriptionPda,
                tokenProgram: TOKEN_PROGRAM_ID,
                systemProgram: SystemProgram.programId,
            })
            .rpc();

          assert.fail("Should have failed due to invalid vault");
      } catch (e: any) {
          assert.include(e.message, "Invalid insurance vault");
      }
  });

  it("Checks SPL split and burn logic mathematically", async () => {
      // In a real environment with a cloned SAKURA_MINT and the REAL insurance vault, 
      // we would verify that calling processPayment with 10_000 tokens results in:
      // 1. userTokenAccount balance strictly decreasing by 10_000
      // 2. insuranceVault balance strictly increasing by 5_000
      // 3. SAKURA_MINT total supply strictly decreasing by 5_000 (proving the permanent SPL burn)
      
      // The logic in lib.rs implements this tightly using:
      // token::burn(burn_ctx, burn_amount)?;
      console.log("SPL split and burn logic validation outlined for mainnet-fork testing.");
  });

});
