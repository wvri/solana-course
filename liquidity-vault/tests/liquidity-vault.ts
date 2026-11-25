import * as anchor from "@anchor-lang/core";
import { Program, BN } from "@anchor-lang/core";
import { Keypair, LAMPORTS_PER_SOL, PublicKey } from "@solana/web3.js";
import {
  TOKEN_PROGRAM_ID,
  createMint,
  createAssociatedTokenAccount,
  getAccount,
  getAssociatedTokenAddressSync,
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
  let userShareAta: PublicKey;

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
    userShareAta = getAssociatedTokenAddressSync(shareMint, user.publicKey);
  });

  const balance = async (ata: PublicKey) =>
    Number((await getAccount(provider.connection, ata)).amount);

  const withdraw = (shares: number, minAssets = 0) =>
    program.methods
      .withdraw(new BN(shares), new BN(minAssets))
      .accountsPartial({
        user: user.publicKey,
        config,
        vaultAuthority,
        assetMint,
        shareMint,
        vaultAsset,
        userAssetAccount: userAssetAta,
        userShareAccount: userShareAta,
        userAccount,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([user])
      .rpc();

  const deposit = (amount: number, minShares = 0) =>
    program.methods
      .deposit(new BN(amount), new BN(minShares))
      .accountsPartial({
        user: user.publicKey,
        config,
        vaultAuthority,
        assetMint,
        shareMint,
        vaultAsset,
        userAssetAccount: userAssetAta,
        userShareAccount: userShareAta,
        userAccount,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .signers([user])
      .rpc();

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

  // ───────────── Неделя 9 ─────────────

  it("deposit: первый депозит выпускает доли 1:1", async () => {
    await deposit(100 * ONE);

    assert.equal(await balance(userShareAta), 100 * ONE);
    assert.equal(await balance(vaultAsset), 100 * ONE);
    const cfg = await program.account.vaultConfig.fetch(config);
    assert.equal(cfg.totalAssets.toNumber(), 100 * ONE);
    assert.equal(cfg.totalShares.toNumber(), 100 * ONE);
    const ua = await program.account.userAccount.fetch(userAccount);
    assert.equal(ua.totalDeposited.toNumber(), 100 * ONE);
  });

  it("deposit: повторный депозит пропорционален", async () => {
    await deposit(50 * ONE);
    assert.equal(await balance(userShareAta), 150 * ONE);
  });

  it("deposit: min_shares_out защищает от проскальзывания", async () => {
    try {
      await deposit(10 * ONE, 11 * ONE);
      assert.fail("должно было упасть");
    } catch (e: any) {
      assert.include(String(e), "SlippageExceeded");
    }
  });

  it("deposit: пауза блокирует депозиты", async () => {
    await program.methods
      .updateSettings(true, new BN(0))
      .accountsPartial({ admin: admin.publicKey, config })
      .rpc();
    try {
      await deposit(1 * ONE);
      assert.fail("должно было упасть");
    } catch (e: any) {
      assert.include(String(e), "VaultPaused");
    }
    await program.methods
      .updateSettings(false, new BN(0))
      .accountsPartial({ admin: admin.publicKey, config })
      .rpc();
  });

  it("deposit: лимит депозитов", async () => {
    await program.methods
      .updateSettings(false, new BN(200 * ONE))
      .accountsPartial({ admin: admin.publicKey, config })
      .rpc();
    try {
      await deposit(60 * ONE); // 150 + 60 > 200
      assert.fail("должно было упасть");
    } catch (e: any) {
      assert.include(String(e), "DepositCapExceeded");
    }
    await program.methods
      .updateSettings(false, new BN(0))
      .accountsPartial({ admin: admin.publicKey, config })
      .rpc();
  });

  // ───────────── Неделя 10 ─────────────

  it("withdraw: сжигает доли и возвращает актив через CPI с подписью PDA", async () => {
    const assetBefore = await balance(userAssetAta);
    await withdraw(50 * ONE);

    assert.equal(await balance(userShareAta), 100 * ONE);
    assert.equal(await balance(vaultAsset), 100 * ONE);
    assert.equal(await balance(userAssetAta), assetBefore + 50 * ONE);
    const cfg = await program.account.vaultConfig.fetch(config);
    assert.equal(cfg.totalShares.toNumber(), 100 * ONE);
    const ua = await program.account.userAccount.fetch(userAccount);
    assert.equal(ua.sharesBurned.toNumber(), 50 * ONE);
  });

  it("withdraw: нельзя сжечь больше долей, чем есть", async () => {
    try {
      await withdraw(101 * ONE);
      assert.fail("должно было упасть");
    } catch (e: any) {
      assert.include(String(e), "InsufficientShares");
    }
  });

  it("withdraw: min_assets_out защищает от проскальзывания", async () => {
    try {
      await withdraw(10 * ONE, 11 * ONE);
      assert.fail("должно было упасть");
    } catch (e: any) {
      assert.include(String(e), "SlippageExceeded");
    }
  });

  it("withdraw: работает даже на паузе и выводит всё", async () => {
    await program.methods
      .updateSettings(true, new BN(0))
      .accountsPartial({ admin: admin.publicKey, config })
      .rpc();
    await withdraw(100 * ONE);

    assert.equal(await balance(userShareAta), 0);
    assert.equal(await balance(vaultAsset), 0);
    const cfg = await program.account.vaultConfig.fetch(config);
    assert.equal(cfg.totalAssets.toNumber(), 0);
    assert.equal(cfg.totalShares.toNumber(), 0);
  });
});
