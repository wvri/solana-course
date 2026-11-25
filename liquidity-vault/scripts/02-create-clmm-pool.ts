/**
 * Неделя 11, шаг 2: создаём CLMM-пул Raydium A/B в devnet.
 *
 *   yarn ts-node scripts/02-create-clmm-pool.ts [initialPrice]
 *
 * initialPrice — цена A в единицах B (по умолчанию 1).
 * Raydium сортирует mint-ы: token_0 < token_1 (побайтово), SDK сам
 * переворачивает цену, если порядок другой.
 */
import Decimal from "decimal.js";
import { PublicKey } from "@solana/web3.js";
import { CLMM_PROGRAM_ID, addr, initSdk, loadState, need, saveState, tx, txVersion } from "./common";

async function main() {
  const state = loadState();
  const raydium = await initSdk();

  const mint1 = await raydium.token.getTokenInfo(need(state, "mintA"));
  const mint2 = await raydium.token.getTokenInfo(need(state, "mintB"));

  // Конфиги комиссий devnet: https://api-v3-devnet.raydium.io/main/clmm-config
  const configs = await raydium.api.getClmmConfigs();
  configs.sort((a, b) => a.tradeFeeRate - b.tradeFeeRate);
  // берём конфиг с комиссией 0.25% (или ближайший), tickSpacing у него 60
  const cfg = configs.find((c) => c.tradeFeeRate === 2500) ?? configs[0];
  console.log(`AMM config: ${cfg.id}  fee=${cfg.tradeFeeRate / 10_000}%  tickSpacing=${cfg.tickSpacing}`);

  const initialPrice = new Decimal(process.argv[2] ?? "1");

  const { execute, extInfo } = await raydium.clmm.createPool({
    programId: CLMM_PROGRAM_ID,
    mint1,
    mint2,
    ammConfig: { ...cfg, id: new PublicKey(cfg.id), fundOwner: "", description: "" },
    initialPrice,
    txVersion,
  });

  const { txId } = await execute({ sendAndConfirm: true });
  const poolId = extInfo.address.id.toString();

  console.log("CLMM pool:", poolId);
  console.log("  pool:  ", addr(poolId));
  console.log("  tx:    ", tx(txId));
  console.log("  mint0: ", extInfo.address.mintA.address);
  console.log("  mint1: ", extInfo.address.mintB.address);
  saveState({ poolId });
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
