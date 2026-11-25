/**
 * Неделя 13, шаг 2: собрать комиссии позиции в vault (liquidity не меняется).
 *
 *   yarn ts-node scripts/07-collect-fees.ts
 *
 * Чтобы комиссии появились, в пуле должны быть обмены — например,
 * несколько раз запустите 05-vault-swap.ts в обе стороны или сделайте swap
 * со своего кошелька в интерфейсе Raydium devnet.
 */
import { Transaction } from "@solana/web3.js";
import { tx } from "./common";
import { balanceOf, buildCollectFees, computeIx, loadContext, readPosition } from "./position-lib";

async function main() {
  const ctx = await loadContext();
  if (!ctx.strategy.hasPosition) throw new Error("У vault нет позиции — запустите 06-open-position.ts");

  const pos = await readPosition(ctx);
  console.log(`Позиция: ticks [${pos.tickLower}, ${pos.tickUpper}), liquidity ${pos.liquidity.toString()}`);

  const [a0, p0] = [await balanceOf(ctx.p.vaultAsset), await balanceOf(ctx.p.vaultPair)];
  const ix = await buildCollectFees(ctx);
  const sig = await ctx.program.provider.sendAndConfirm!(new Transaction().add(computeIx, ix));
  const [a1, p1] = [await balanceOf(ctx.p.vaultAsset), await balanceOf(ctx.p.vaultPair)];

  console.log(`Комиссии: asset +${a1.sub(a0).toString()}, pair +${p1.sub(p0).toString()}`);
  console.log("collect_fees tx:", tx(sig));
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
