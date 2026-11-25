/**
 * Swap токенов vault через CLMM-пул Raydium (инструкция vault `swap`).
 * Используется в 05-vault-swap.ts (нед. 12) и 08-rebalance.ts (нед. 13).
 */
import { BN } from "@anchor-lang/core";
import {
  TickArrayBitmapExtensionLayout,
  getPdaExBitmapAccount,
  swapInternal,
} from "@raydium-io/raydium-sdk-v2";
import { ComputeBudgetProgram, PublicKey } from "@solana/web3.js";
import { TOKEN_2022_PROGRAM_ID, TOKEN_PROGRAM_ID, getAccount } from "@solana/spl-token";
import { CLMM_PROGRAM_ID, MEMO_PROGRAM_ID, connection, initSdk, loadState, need, owner, vaultPdas, vaultProgram } from "./common";

export type SwapResult = { sig: string; amountIn: BN; expectedOut: BN; minAmountOut: BN; received: bigint };

/**
 * @param amountIn     сколько отдать, в сырых единицах токена
 * @param assetToPair  true = asset → pair, false = pair → asset
 * @param slippageBps  допуск, 100 = 1%
 */
export async function vaultSwap(amountIn: BN, assetToPair: boolean, slippageBps: number): Promise<SwapResult> {
  const state = loadState();
  const program = vaultProgram();
  const raydium = await initSdk();
  const assetMint = new PublicKey(need(state, "mintA"));
  const poolId = new PublicKey(need(state, "poolId"));
  const p = vaultPdas(program.programId, assetMint);
  const strategy = await program.account.strategy.fetch(p.strategy);

  // zeroForOne = входной токен — token_0 пула
  const inputIsToken0 = assetToPair ? strategy.assetIsToken0 : !strategy.assetIsToken0;
  const { rpcData, configInfo, tickArrays } = await raydium.clmm.getSwapPoolInfo(poolId, inputIsToken0);

  // 1. симуляция: ожидаемый выход и нужные tick arrays
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

  // 2. ограничение на минимальную сумму получения
  const expectedOut: BN = sim.amountCalculated;
  const minAmountOut = expectedOut.muln(10_000 - slippageBps).divn(10_000);

  // 3. вызов vault.swap
  const [inputPoolVault, outputPoolVault] = inputIsToken0
    ? [rpcData.vaultA, rpcData.vaultB]
    : [rpcData.vaultB, rpcData.vaultA];
  const outAcc = assetToPair ? p.vaultPair : p.vaultAsset;
  const before = (await getAccount(connection, outAcc)).amount;

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

  const received = (await getAccount(connection, outAcc)).amount - before;
  return { sig, amountIn, expectedOut, minAmountOut, received };
}
