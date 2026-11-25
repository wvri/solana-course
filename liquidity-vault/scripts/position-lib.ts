/**
 * Общая логика недели 13: открытие / сбор комиссий / вывод / закрытие позиции
 * vault в CLMM-пуле Raydium. Все действия идут через инструкции программы vault,
 * которая делает CPI в Raydium с подписью PDA vault_authority.
 */
import { BN } from "@anchor-lang/core";
import {
  LiquidityMathUtil,
  PersonalPositionLayout,
  TickArrayUtil,
  TickUtil,
  getPdaExBitmapAccount,
  getPdaPersonalPositionAddress,
  getPdaProtocolPositionAddress,
  getPdaTickArrayAddress,
} from "@raydium-io/raydium-sdk-v2";
import { ComputeBudgetProgram, Keypair, PublicKey, TransactionInstruction } from "@solana/web3.js";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  TOKEN_2022_PROGRAM_ID,
  TOKEN_PROGRAM_ID,
  getAccount,
  getAssociatedTokenAddressSync,
} from "@solana/spl-token";
import Decimal from "decimal.js";
import {
  CLMM_PROGRAM_ID, MEMO_PROGRAM_ID, connection, initSdk, loadState, need, owner, vaultPdas, vaultProgram,
} from "./common";

export const computeIx = ComputeBudgetProgram.setComputeUnitLimit({ units: 800_000 });

/** Всё, что нужно для работы с позицией: программа, PDA, стратегия, пул. */
export async function loadContext() {
  const state = loadState();
  const program = vaultProgram();
  const raydium = await initSdk();
  const assetMint = new PublicKey(need(state, "mintA"));
  const poolId = new PublicKey(need(state, "poolId"));
  const p = vaultPdas(program.programId, assetMint);
  const strategy = await program.account.strategy.fetch(p.strategy);
  const pool = await raydium.clmm.getRpcClmmPoolInfo({ poolId });
  return { program, raydium, assetMint, poolId, p, strategy, pool };
}
export type Ctx = Awaited<ReturnType<typeof loadContext>>;

export const balanceOf = async (a: PublicKey) => new BN((await getAccount(connection, a)).amount.toString());

/** token accounts vault в порядке token_0 / token_1 пула. */
export function vaultTokenAccounts(ctx: Ctx): [PublicKey, PublicKey] {
  return ctx.strategy.assetIsToken0 ? [ctx.p.vaultAsset, ctx.p.vaultPair] : [ctx.p.vaultPair, ctx.p.vaultAsset];
}

/** Тики диапазона ±rangePct% вокруг текущей цены, кратные tickSpacing. */
export function rangeAroundPrice(ctx: Ctx, rangePct: number): [number, number] {
  const { pool } = ctx;
  const price = new Decimal(pool.currentPrice);
  const toTick = (p: Decimal) =>
    TickUtil.toTickIndex(TickUtil.priceToTick(p, pool.mintDecimalsA, pool.mintDecimalsB), pool.tickSpacing);
  let lower = toTick(price.mul(1 - rangePct / 100));
  let upper = toTick(price.mul(1 + rangePct / 100));
  if (upper <= lower) upper = lower + pool.tickSpacing;
  // текущий тик обязан быть внутри [lower, upper)
  if (pool.tickCurrent < lower) lower -= pool.tickSpacing;
  if (pool.tickCurrent >= upper) upper += pool.tickSpacing;
  return [lower, upper];
}

function tickArrays(ctx: Ctx, tickLower: number, tickUpper: number) {
  const startLower = TickArrayUtil.getTickArrayStartIndex(tickLower, ctx.pool.tickSpacing);
  const startUpper = TickArrayUtil.getTickArrayStartIndex(tickUpper, ctx.pool.tickSpacing);
  return {
    startLower,
    startUpper,
    tickArrayLower: getPdaTickArrayAddress(CLMM_PROGRAM_ID, ctx.poolId, startLower).publicKey,
    tickArrayUpper: getPdaTickArrayAddress(CLMM_PROGRAM_ID, ctx.poolId, startUpper).publicKey,
  };
}

const bitmapExt = (ctx: Ctx) => getPdaExBitmapAccount(CLMM_PROGRAM_ID, ctx.poolId).publicKey;
const writable = (pubkey: PublicKey) => ({ pubkey, isSigner: false, isWritable: true });

// ───────────── open_position ─────────────

export async function buildOpenPosition(ctx: Ctx, tickLower: number, tickUpper: number) {
  const { program, pool, p, poolId } = ctx;
  const [acc0, acc1] = vaultTokenAccounts(ctx);
  const [bal0, bal1] = [await balanceOf(acc0), await balanceOf(acc1)];

  // Ликвидность из доступных балансов vault (используется весь лимитирующий токен).
  const sqrtCur = pool.sqrtPriceX64;
  const sqrtLower = TickUtil.getSqrtPriceAtTick(tickLower);
  const sqrtUpper = TickUtil.getSqrtPriceAtTick(tickUpper);
  const liquidity = LiquidityMathUtil.getLiquidityFromAmounts(sqrtCur, sqrtLower, sqrtUpper, bal0, bal1)
    .muln(99)
    .divn(100); // запас 1% на движение цены
  if (liquidity.isZero()) throw new Error("В vault нет токенов для позиции — сделайте deposit/swap");
  const required = LiquidityMathUtil.getAmountsForLiquidity(sqrtCur, sqrtLower, sqrtUpper, liquidity, true);

  const nftMint = Keypair.generate();
  const ta = tickArrays(ctx, tickLower, tickUpper);
  const ix = await program.methods
    .openPosition(
      tickLower,
      tickUpper,
      ta.startLower,
      ta.startUpper,
      liquidity,
      BN.min(bal0, required.amountA.muln(101).divn(100)), // amount_0_max (защита +1%)
      BN.min(bal1, required.amountB.muln(101).divn(100)), // amount_1_max
      null
    )
    .accountsPartial({
      admin: owner.publicKey,
      config: p.config,
      vaultAuthority: p.vaultAuthority,
      strategy: p.strategy,
      vaultAsset: p.vaultAsset,
      vaultPair: p.vaultPair,
      clmmProgram: CLMM_PROGRAM_ID,
      positionNftMint: nftMint.publicKey,
      positionNftAccount: getAssociatedTokenAddressSync(
        nftMint.publicKey, p.vaultAuthority, true, TOKEN_2022_PROGRAM_ID
      ),
      poolState: poolId,
      protocolPosition: getPdaProtocolPositionAddress(CLMM_PROGRAM_ID, poolId, tickLower, tickUpper).publicKey,
      tickArrayLower: ta.tickArrayLower,
      tickArrayUpper: ta.tickArrayUpper,
      personalPosition: getPdaPersonalPositionAddress(CLMM_PROGRAM_ID, nftMint.publicKey).publicKey,
      tokenVault0: pool.vaultA,
      tokenVault1: pool.vaultB,
      vault0Mint: pool.mintA,
      vault1Mint: pool.mintB,
      tokenProgram: TOKEN_PROGRAM_ID,
      associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
      tokenProgram2022: TOKEN_2022_PROGRAM_ID,
    })
    .remainingAccounts([writable(bitmapExt(ctx))])
    .instruction();
  return { ix, nftMint, liquidity };
}

// ───────────── decrease_liquidity / collect_fees ─────────────

export async function readPosition(ctx: Ctx) {
  const nftMint = ctx.strategy.positionNftMint;
  const addr = getPdaPersonalPositionAddress(CLMM_PROGRAM_ID, nftMint).publicKey;
  const info = await connection.getAccountInfo(addr);
  if (!info) throw new Error("Позиция не найдена");
  return PersonalPositionLayout.decode(info.data);
}

function manageAccounts(ctx: Ctx) {
  const { p, pool, poolId, strategy } = ctx;
  const ta = tickArrays(ctx, strategy.tickLower, strategy.tickUpper);
  return {
    admin: owner.publicKey,
    config: p.config,
    vaultAuthority: p.vaultAuthority,
    strategy: p.strategy,
    vaultAsset: p.vaultAsset,
    vaultPair: p.vaultPair,
    clmmProgram: CLMM_PROGRAM_ID,
    nftAccount: getAssociatedTokenAddressSync(
      strategy.positionNftMint, p.vaultAuthority, true, TOKEN_2022_PROGRAM_ID
    ),
    personalPosition: getPdaPersonalPositionAddress(CLMM_PROGRAM_ID, strategy.positionNftMint).publicKey,
    poolState: poolId,
    protocolPosition: getPdaProtocolPositionAddress(
      CLMM_PROGRAM_ID, poolId, strategy.tickLower, strategy.tickUpper
    ).publicKey,
    tokenVault0: pool.vaultA,
    tokenVault1: pool.vaultB,
    tickArrayLower: ta.tickArrayLower,
    tickArrayUpper: ta.tickArrayUpper,
    vault0Mint: pool.mintA,
    vault1Mint: pool.mintB,
    tokenProgram: TOKEN_PROGRAM_ID,
    tokenProgram2022: TOKEN_2022_PROGRAM_ID,
    memoProgram: MEMO_PROGRAM_ID,
  };
}

export function buildCollectFees(ctx: Ctx): Promise<TransactionInstruction> {
  return ctx.program.methods
    .collectFees()
    .accountsPartial(manageAccounts(ctx))
    .remainingAccounts([writable(bitmapExt(ctx))])
    .instruction();
}

/** Вывести всю ликвидность; min = ожидаемое × (1 − slippage). */
export async function buildDecreaseAll(ctx: Ctx, slippageBps = 100) {
  const pos = await readPosition(ctx);
  const { amountA, amountB } = LiquidityMathUtil.getAmountsForLiquidity(
    ctx.pool.sqrtPriceX64,
    TickUtil.getSqrtPriceAtTick(pos.tickLower),
    TickUtil.getSqrtPriceAtTick(pos.tickUpper),
    pos.liquidity,
    false
  );
  const k = 10_000 - slippageBps;
  const ix = await ctx.program.methods
    .decreaseLiquidity(pos.liquidity, amountA.muln(k).divn(10_000), amountB.muln(k).divn(10_000))
    .accountsPartial(manageAccounts(ctx))
    .remainingAccounts([writable(bitmapExt(ctx))])
    .instruction();
  return { ix, liquidity: pos.liquidity };
}

// ───────────── close_position / check_range ─────────────

export function buildClosePosition(ctx: Ctx): Promise<TransactionInstruction> {
  const { p, poolId, strategy } = ctx;
  return ctx.program.methods
    .closePosition()
    .accountsPartial({
      admin: owner.publicKey,
      config: p.config,
      vaultAuthority: p.vaultAuthority,
      strategy: p.strategy,
      clmmProgram: CLMM_PROGRAM_ID,
      positionNftMint: strategy.positionNftMint,
      positionNftAccount: getAssociatedTokenAddressSync(
        strategy.positionNftMint, p.vaultAuthority, true, TOKEN_2022_PROGRAM_ID
      ),
      personalPosition: getPdaPersonalPositionAddress(CLMM_PROGRAM_ID, strategy.positionNftMint).publicKey,
      poolState: poolId,
      tokenProgram2022: TOKEN_2022_PROGRAM_ID,
    })
    .instruction();
}

export function buildCheckRange(ctx: Ctx, requireOutOfRange: boolean): Promise<TransactionInstruction> {
  return ctx.program.methods
    .checkRange(requireOutOfRange)
    .accountsPartial({ config: ctx.p.config, strategy: ctx.p.strategy, poolState: ctx.poolId })
    .instruction();
}

/** Цена token_0 в единицах token_1 (сырые единицы, без decimals) из sqrtPriceX64. */
export function rawPrice(ctx: Ctx): Decimal {
  return new Decimal(ctx.pool.sqrtPriceX64.toString()).div(new Decimal(2).pow(64)).pow(2);
}

export function isInRange(ctx: Ctx): boolean {
  const t = ctx.pool.tickCurrent;
  return ctx.strategy.tickLower <= t && t < ctx.strategy.tickUpper;
}
