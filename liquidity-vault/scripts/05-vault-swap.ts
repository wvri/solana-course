/**
 * Неделя 12: swap токенов vault через CLMM-пул Raydium с защитой от проскальзывания.
 *
 *   yarn ts-node scripts/05-vault-swap.ts [amount] [slippageBps] [direction]
 *     amount       — сколько отдать, в целых токенах (по умолчанию 1000)
 *     slippageBps  — допуск, 100 = 1% (по умолчанию 50 = 0.5%)
 *     direction    — "a2b" (asset → pair, по умолчанию) или "b2a"
 *
 * 1) Симулируем swap в Raydium SDK → ожидаемый выход и нужные tick arrays.
 * 2) min_amount_out = ожидаемый × (1 − slippage).
 * 3) Вызываем инструкцию vault `swap`: программа делает CPI swap_v2,
 *    подписывая от имени PDA vault_authority.
 */
import { BN } from "@anchor-lang/core";
import {
  TickArrayBitmapExtensionLayout,
  getPdaExBitmapAccount,
  swapInternal,
} from "@raydium-io/raydium-sdk-v2";
import { ComputeBudgetProgram, PublicKey } from "@solana/web3.js";
import { TOKEN_2022_PROGRAM_ID, TOKEN_PROGRAM_ID, getAccount } from "@solana/spl-token";
import {
  CLMM_PROGRAM_ID, MEMO_PROGRAM_ID, connection, initSdk, loadState, need, owner, tx, vaultPdas, vaultProgram,
} from "./common";

async function main() {
  const amountUi = Number(process.argv[2] ?? "1000");
  const slippageBps = Number(process.argv[3] ?? "50");
  const assetToPair = (process.argv[4] ?? "a2b") !== "b2a";

  const state = loadState();
  const program = vaultProgram();
  const raydium = await initSdk();
  const assetMint = new PublicKey(need(state, "mintA"));
  const poolId = new PublicKey(need(state, "poolId"));
  const p = vaultPdas(program.programId, assetMint);
  const strategy = await program.account.strategy.fetch(p.strategy);

  // zeroForOne = входной токен — token_0 пула
  const inputIsToken0 = assetToPair ? strategy.assetIsToken0 : !strategy.assetIsToken0;
  const { poolInfo, rpcData, configInfo, tickArrays } = await raydium.clmm.getSwapPoolInfo(poolId, inputIsToken0);
  const inputDecimals = inputIsToken0 ? poolInfo.mintA.decimals : poolInfo.mintB.decimals;
  const amountIn = new BN(amountUi).mul(new BN(10).pow(new BN(inputDecimals)));

  // ── 1. симуляция ──
  const bitmapExt = getPdaExBitmapAccount(CLMM_PROGRAM_ID, poolId).publicKey;
  const bitmapInfo = await connection.getAccountInfo(bitmapExt);
  const sim = swapInternal({
    programId: CLMM_PROGRAM_ID,
    poolId,
    poolInfo: rpcData,
    tickArrays,
    configInfo,
    tickarrayBitmapExtension: TickArrayBitmapExtensionLayout.decode(bitmapInfo!.data),
    amountSpecified: amountIn,
    sqrtPriceLimitX64: new BN(0),
    zeroForOne: inputIsToken0,
    isBaseInput: true,
    blockTimestamp: Math.floor(Date.now() / 1000),
    includeExtraTickArrays: true,
  });

  // ── 2. ограничение на минимальную сумму получения ──
  const expectedOut: BN = sim.amountCalculated;
  const minAmountOut = expectedOut.muln(10_000 - slippageBps).divn(10_000);
  console.log(`Направление: ${assetToPair ? "asset → pair" : "pair → asset"}`);
  console.log(`amount_in:      ${amountIn.toString()}`);
  console.log(`ожидаемо:       ${expectedOut.toString()}`);
  console.log(`min_amount_out: ${minAmountOut.toString()}  (slippage ${slippageBps / 100}%)`);

  // ── 3. вызов vault.swap ──
  const [inputPoolVault, outputPoolVault] = inputIsToken0
    ? [rpcData.vaultA, rpcData.vaultB]
    : [rpcData.vaultB, rpcData.vaultA];
  const balanceOf = async (a: PublicKey) => (await getAccount(connection, a)).amount;
  const outAcc = assetToPair ? p.vaultPair : p.vaultAsset;
  const before = await balanceOf(outAcc);

  const sig = await program.methods
    .swap(amountIn, minAmountOut, new BN(0), assetToPair)
    .accountsPartial({
      admin: owner.publicKey,
      config: p.config,
      vaultAuthority: p.vaultAuthority,
      strategy: p.strategy,
      vaultAsset: p.vaultAsset,
      vaultPair: p.vaultPair,
      assetMint,
      pairMint: strategy.pairMint,
      clmmProgram: CLMM_PROGRAM_ID,
      ammConfig: rpcData.configId,
      poolState: poolId,
      inputPoolVault,
      outputPoolVault,
      observationState: rpcData.observationId,
      tokenProgram: TOKEN_PROGRAM_ID,
      tokenProgram2022: TOKEN_2022_PROGRAM_ID,
      memoProgram: MEMO_PROGRAM_ID,
    })
    .remainingAccounts(sim.accounts.map((pubkey: PublicKey) => ({ pubkey, isSigner: false, isWritable: true })))
    .preInstructions([ComputeBudgetProgram.setComputeUnitLimit({ units: 600_000 })])
    .rpc();

  const received = (await balanceOf(outAcc)) - before;
  console.log(`получено:       ${received}`);
  console.log("swap tx:", tx(sig));
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
