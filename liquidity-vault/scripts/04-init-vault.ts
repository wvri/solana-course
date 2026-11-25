/**
 * Неделя 11, шаг 4: разворачиваем vault для Token A в devnet и привязываем
 * его к CLMM-пулу A/B (set_strategy), затем вносим депозит.
 *
 *   anchor build && anchor deploy --provider.cluster devnet
 *   yarn ts-node scripts/04-init-vault.ts
 */
import { BN } from "@anchor-lang/core";
import { LAMPORTS_PER_SOL, PublicKey } from "@solana/web3.js";
import { TOKEN_PROGRAM_ID, getAssociatedTokenAddressSync } from "@solana/spl-token";
import { CLMM_PROGRAM_ID, addr, connection, loadState, need, owner, saveState, tx, vaultPdas, vaultProgram } from "./common";

const DEPOSIT = 10_000 * 10 ** 6; // 10 000 Token A

async function main() {
  const state = loadState();
  const program = vaultProgram();
  const assetMint = new PublicKey(need(state, "mintA"));
  const pairMint = new PublicKey(need(state, "mintB"));
  const poolState = new PublicKey(need(state, "poolId"));
  const p = vaultPdas(program.programId, assetMint);

  if (!(await connection.getAccountInfo(p.config))) {
    const sig = await program.methods
      // authority получает 0.1 SOL на rent позиции Raydium (нед. 13)
      .initializeVault(new BN(0), new BN(0.1 * LAMPORTS_PER_SOL))
      .accountsPartial({
        admin: owner.publicKey,
        assetMint,
        config: p.config,
        vaultAuthority: p.vaultAuthority,
        shareMint: p.shareMint,
        vaultAsset: p.vaultAsset,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .rpc();
    console.log("initialize_vault:", tx(sig));
  }

  if (!(await connection.getAccountInfo(p.strategy))) {
    const sig = await program.methods
      .setStrategy()
      .accountsPartial({
        admin: owner.publicKey,
        config: p.config,
        vaultAuthority: p.vaultAuthority,
        strategy: p.strategy,
        poolState,
        pairMint,
        vaultPair: p.vaultPair,
        clmmProgram: CLMM_PROGRAM_ID,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .rpc();
    console.log("set_strategy:", tx(sig));
  }

  const userAccount = p.userAccount(owner.publicKey);
  if (!(await connection.getAccountInfo(userAccount))) {
    const sig = await program.methods
      .initUser()
      .accountsPartial({ owner: owner.publicKey, config: p.config, userAccount })
      .rpc();
    console.log("init_user:", tx(sig));
  }

  const sig = await program.methods
    .deposit(new BN(DEPOSIT), new BN(0))
    .accountsPartial({
      user: owner.publicKey,
      config: p.config,
      vaultAuthority: p.vaultAuthority,
      assetMint,
      shareMint: p.shareMint,
      vaultAsset: p.vaultAsset,
      userAssetAccount: getAssociatedTokenAddressSync(assetMint, owner.publicKey),
      userShareAccount: getAssociatedTokenAddressSync(p.shareMint, owner.publicKey),
      userAccount,
      tokenProgram: TOKEN_PROGRAM_ID,
    })
    .rpc();
  console.log("deposit:", tx(sig));

  const strategy = await program.account.strategy.fetch(p.strategy);
  console.log("VaultConfig:", addr(p.config));
  console.log("Strategy:   ", addr(p.strategy), `asset_is_token_0=${strategy.assetIsToken0}`);
  saveState({ vaultConfig: p.config.toBase58() });
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
