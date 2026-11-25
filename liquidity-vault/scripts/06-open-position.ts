/**
 * Неделя 13, шаг 1: открыть позицию vault в выбранном ценовом диапазоне.
 *
 *   yarn ts-node scripts/06-open-position.ts [rangePct]
 *     rangePct — ширина диапазона вокруг текущей цены, ±% (по умолчанию 5)
 *
 * Перед этим в vault должны быть оба токена: deposit (04) + swap части A → B (05).
 */
import { Transaction } from "@solana/web3.js";
import { addr, saveState, tx } from "./common";
import { buildOpenPosition, computeIx, loadContext, rangeAroundPrice } from "./position-lib";

async function main() {
  const rangePct = Number(process.argv[2] ?? "5");
  const ctx = await loadContext();
  if (ctx.strategy.hasPosition) throw new Error("У vault уже есть позиция — см. 07/08");

  const [tickLower, tickUpper] = rangeAroundPrice(ctx, rangePct);
  console.log(`Текущий тик: ${ctx.pool.tickCurrent}, цена: ${ctx.pool.currentPrice}`);
  console.log(`Диапазон ±${rangePct}%: ticks [${tickLower}, ${tickUpper})`);

  const { ix, nftMint, liquidity } = await buildOpenPosition(ctx, tickLower, tickUpper);
  const sig = await ctx.program.provider.sendAndConfirm!(new Transaction().add(computeIx, ix), [nftMint]);

  console.log("liquidity:", liquidity.toString());
  console.log("position NFT:", addr(nftMint.publicKey));
  console.log("open_position tx:", tx(sig));
  saveState({ positionNftMint: nftMint.publicKey.toBase58() });
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
