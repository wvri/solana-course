/**
 * Неделя 13, шаг 4: переоценка vault (NAV) и запись в total_assets.
 *
 *   yarn ts-node scripts/09-report-assets.ts [--dry]
 *
 * NAV (в единицах asset) = свободный asset
 *                        + свободный pair × цена
 *                        + токены в позиции CLMM (по текущей цене)
 * Программа не даст изменить total_assets больше чем на ±20% за раз.
 */
import { BN } from "@anchor-lang/core";
import { LiquidityMathUtil, TickUtil } from "@raydium-io/raydium-sdk-v2";
import Decimal from "decimal.js";
import { owner, tx } from "./common";
import { balanceOf, loadContext, rawPrice, readPosition } from "./position-lib";

async function main() {
  const dry = process.argv.includes("--dry");
  const ctx = await loadContext();
  const assetIs0 = ctx.strategy.assetIsToken0;
  const price = rawPrice(ctx); // token_1 за 1 token_0

  let asset = new Decimal((await balanceOf(ctx.p.vaultAsset)).toString());
  let pair = new Decimal((await balanceOf(ctx.p.vaultPair)).toString());

  if (ctx.strategy.hasPosition) {
    const pos = await readPosition(ctx);
    const { amountA, amountB } = LiquidityMathUtil.getAmountsForLiquidity(
      ctx.pool.sqrtPriceX64,
      TickUtil.getSqrtPriceAtTick(pos.tickLower),
      TickUtil.getSqrtPriceAtTick(pos.tickUpper),
      pos.liquidity,
      false
    );
    const [inAsset, inPair] = assetIs0 ? [amountA, amountB] : [amountB, amountA];
    asset = asset.add(inAsset.toString());
    pair = pair.add(inPair.toString());
  }

  // pair → asset: если asset = token_0, то pair = token_1 и делим на цену
  const pairInAsset = assetIs0 ? pair.div(price) : pair.mul(price);
  const nav = asset.add(pairInAsset).floor();

  const cfg = await ctx.program.account.vaultConfig.fetch(ctx.p.config);
  console.log(`asset: ${asset.toFixed(0)}, pair: ${pair.toFixed(0)} (≈ ${pairInAsset.toFixed(0)} asset)`);
  console.log(`total_assets: ${cfg.totalAssets.toString()} → NAV ${nav.toFixed(0)}`);
  console.log(`цена доли: ${nav.div(Decimal.max(1, cfg.totalShares.toString())).toFixed(6)} asset/share`);
  if (dry) return;

  const sig = await ctx.program.methods
    .reportAssets(new BN(nav.toFixed(0)))
    .accountsPartial({ admin: owner.publicKey, config: ctx.p.config })
    .rpc();
  console.log("report_assets tx:", tx(sig));
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
