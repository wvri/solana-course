/**
 * Неделя 11, шаг 3: добавляем стартовую ликвидность в пул со своего кошелька
 * (широкий диапазон 0.5–2.0 вокруг цены 1), чтобы в пуле можно было делать swap.
 *
 *   yarn ts-node scripts/03-seed-liquidity.ts
 */
import BN from "bn.js";
import Decimal from "decimal.js";
import { TickUtil } from "@raydium-io/raydium-sdk-v2";
import { initSdk, loadState, need, tx, txVersion } from "./common";

const SEED_AMOUNT_A = 100_000; // в целых токенах
const [PRICE_LOWER, PRICE_UPPER] = [0.5, 2.0];

async function main() {
  const raydium = await initSdk();
  const { poolInfo, poolKeys } = await raydium.clmm.getPoolInfoFromRpc(need(loadState(), "poolId"));
  const { mintA, mintB } = poolInfo;
  const spacing = poolInfo.config.tickSpacing;

  const toTick = (p: number) =>
    TickUtil.toTickIndex(TickUtil.priceToTick(new Decimal(p), mintA.decimals, mintB.decimals), spacing);
  const tickLower = toTick(PRICE_LOWER);
  const tickUpper = toTick(PRICE_UPPER);
  console.log(`Цена сейчас: ${poolInfo.price}`);
  console.log(`Диапазон: ticks [${tickLower}, ${tickUpper}]`);

  const baseAmount = new BN(SEED_AMOUNT_A).mul(new BN(10).pow(new BN(mintA.decimals)));
  const { execute, extInfo } = await raydium.clmm.openPositionFromBase({
    poolInfo,
    poolKeys,
    ownerInfo: { useSOLBalance: false },
    tickLower,
    tickUpper,
    base: "MintA",
    baseAmount,
    otherAmountMax: baseAmount.muln(3), // верхняя граница по B (защита)
    nft2022: true,
    txVersion,
  });
  const { txId } = await execute({ sendAndConfirm: true });

  console.log("Ликвидность добавлена");
  console.log("  position NFT:", extInfo.nftMint.toBase58());
  console.log("  tx:", tx(txId));
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
