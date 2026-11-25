/**
 * Неделя 13, шаг 3: переставить позицию, если цена вышла из диапазона.
 *
 *   yarn ts-node scripts/08-rebalance.ts [rangePct] [--force]
 *
 * Алгоритм (keeper):
 *   tx1: check_range(require_out_of_range = true)  ← если цена в диапазоне, tx откатится
 *        decrease_liquidity(всё, min с допуском 1%)  ← ликвидность + комиссии → vault
 *        close_position                              ← NFT сжигается, rent → vault_authority
 *   swap: выравниваем доли токенов ~50/50 по стоимости (vault.swap)
 *   tx2: open_position в новом диапазоне ±rangePct% вокруг текущей цены
 *
 * Чтобы увести цену из диапазона для демонстрации: несколько крупных swap
 * в одну сторону (05-vault-swap.ts с большим amount).
 */
import { BN } from "@anchor-lang/core";
import { Transaction } from "@solana/web3.js";
import Decimal from "decimal.js";
import { saveState, tx } from "./common";
import {
  balanceOf, buildCheckRange, buildClosePosition, buildDecreaseAll, buildOpenPosition, computeIx,
  isInRange, loadContext, rangeAroundPrice, rawPrice, vaultTokenAccounts,
} from "./position-lib";
import { vaultSwap } from "./swap-lib";

async function main() {
  const rangePct = Number(process.argv.find((a) => /^\d+(\.\d+)?$/.test(a)) ?? "5");
  const force = process.argv.includes("--force");

  let ctx = await loadContext();
  if (!ctx.strategy.hasPosition) throw new Error("У vault нет позиции — запустите 06-open-position.ts");
  const { tickLower, tickUpper } = ctx.strategy;
  console.log(`Тик пула: ${ctx.pool.tickCurrent}, позиция: [${tickLower}, ${tickUpper})`);

  if (isInRange(ctx) && !force) {
    console.log("Цена внутри диапазона — перестановка не нужна.");
    return;
  }

  // ── tx1: проверка + вывод ликвидности + закрытие ──
  const { ix: decreaseIx } = await buildDecreaseAll(ctx, 100);
  const tx1 = new Transaction().add(
    computeIx,
    await buildCheckRange(ctx, !force),
    decreaseIx,
    await buildClosePosition(ctx)
  );
  const sig1 = await ctx.program.provider.sendAndConfirm!(tx1);
  console.log("check_range + decrease_liquidity + close_position:", tx(sig1));

  // ── выравнивание долей 50/50 по стоимости ──
  ctx = await loadContext();
  const [acc0, acc1] = vaultTokenAccounts(ctx);
  const bal0 = new Decimal((await balanceOf(acc0)).toString());
  const bal1 = new Decimal((await balanceOf(acc1)).toString());
  const price = rawPrice(ctx); // token_1 за 1 token_0
  const value0 = bal0.mul(price); // в единицах token_1
  const excess1 = bal1.sub(value0).div(2); // >0: лишний token_1, <0: лишний token_0
  const assetIs0 = ctx.strategy.assetIsToken0;
  if (excess1.abs().gt(bal1.add(value0).mul(0.01))) {
    // продаём половину излишка
    const sell0 = excess1.isNegative();
    const amount = sell0 ? excess1.abs().div(price) : excess1;
    const assetToPair = sell0 === assetIs0; // продаём asset, если он и есть лишний токен
    const r = await vaultSwap(new BN(amount.floor().toFixed()), assetToPair, 100);
    console.log(`swap ${assetToPair ? "asset → pair" : "pair → asset"}: ${r.amountIn} → ${r.received}`, tx(r.sig));
    ctx = await loadContext();
  }

  // ── tx2: новая позиция вокруг текущей цены ──
  const [newLower, newUpper] = rangeAroundPrice(ctx, rangePct);
  const { ix: openIx, nftMint } = await buildOpenPosition(ctx, newLower, newUpper);
  const sig2 = await ctx.program.provider.sendAndConfirm!(new Transaction().add(computeIx, openIx), [nftMint]);
  console.log(`Новая позиция [${newLower}, ${newUpper}):`, tx(sig2));
  saveState({ positionNftMint: nftMint.publicKey.toBase58() });
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
