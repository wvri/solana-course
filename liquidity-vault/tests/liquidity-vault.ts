import * as anchor from "@anchor-lang/core";
import { Program, BN } from "@anchor-lang/core";
import { Keypair, LAMPORTS_PER_SOL, PublicKey } from "@solana/web3.js";
import {
  TOKEN_PROGRAM_ID,
  createMint,
  createAssociatedTokenAccount,
  mintTo,
} from "@solana/spl-token";
import { assert } from "chai";
import { LiquidityVault } from "../target/types/liquidity_vault";

const DECIMALS = 6;
const ONE = 10 ** DECIMALS;

describe("liquidity-vault", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const program = anchor.workspace.LiquidityVault as Program<LiquidityVault>;
  const admin = (provider.wallet as anchor.Wallet).payer;
  const user = Keypair.generate();

  let assetMint: PublicKey;
  let config: PublicKey;
  let vaultAuthority: PublicKey;
  let shareMint: PublicKey;
  let vaultAsset: PublicKey;
  let userAccount: PublicKey;
  let userAssetAta: PublicKey;

  const pda = (seeds: (Buffer | Uint8Array)[]) =>
    PublicKey.findProgramAddressSync(seeds, program.programId)[0];

  before(async () => {
    const sig = await provider.connection.requestAirdrop(user.publicKey, 2 * LAMPORTS_PER_SOL);
    await provider.connection.confirmTransaction(sig, "confirmed");

    assetMint = await createMint(provider.connection, admin, admin.publicKey, null, DECIMALS);
    userAssetAta = await createAssociatedTokenAccount(
      provider.connection, admin, assetMint, user.publicKey
    );
    await mintTo(provider.connection, admin, assetMint, userAssetAta, admin, 1_000 * ONE);

    config = pda([Buffer.from("config"), assetMint.toBuffer()]);
    vaultAuthority = pda([Buffer.from("authority"), config.toBuffer()]);
    shareMint = pda([Buffer.from("shares"), config.toBuffer()]);
    vaultAsset = pda([Buffer.from("vault_asset"), config.toBuffer()]);
    userAccount = pda([Buffer.from("user"), config.toBuffer(), user.publicKey.toBuffer()]);
  });

  // ───────────── Неделя 8 ─────────────

  it("initialize_vault: создаёт config, share mint и vault", async () => {
    await program.methods
      .initializeVault(new BN(0), new BN(0.05 * LAMPORTS_PER_SOL))
      .accountsPartial({
        admin: admin.publicKey,
        assetMint,
        config,
        vaultAuthority,
        shareMint,
        vaultAsset,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .rpc();

    const cfg = await program.account.vaultConfig.fetch(config);
    assert.ok(cfg.admin.equals(admin.publicKey));
    assert.ok(cfg.shareMint.equals(shareMint));
    assert.equal(cfg.totalShares.toNumber(), 0);
    assert.isFalse(cfg.paused);
  });

  it("init_user: создаёт PDA пользователя", async () => {
    await program.methods
      .initUser()
      .accountsPartial({ owner: user.publicKey, config, userAccount })
      .signers([user])
      .rpc();

    const ua = await program.account.userAccount.fetch(userAccount);
    assert.ok(ua.owner.equals(user.publicKey));
    assert.ok(ua.vault.equals(config));
  });

  it("update_settings: чужой кошелёк не может менять настройки", async () => {
    try {
      await program.methods
        .updateSettings(true, new BN(0))
        .accountsPartial({ admin: user.publicKey, config })
        .signers([user])
        .rpc();
      assert.fail("должно было упасть");
    } catch (e: any) {
      assert.include(String(e), "ConstraintHasOne");
    }
  });
});
