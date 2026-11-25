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
 * Логика вынесена в swap-lib.ts (её же использует перестановка позиции, нед. 13).
 */
import { BN } from "@anchor-lang/core";
import { tx } from "./common";
import { vaultSwap } from "./swap-lib";

const DECIMALS = 6; // оба тестовых токена из 01-create-tokens.ts

async function main() {
  const amountUi = Number(process.argv[2] ?? "1000");
  const slippageBps = Number(process.argv[3] ?? "50");
  const assetToPair = (process.argv[4] ?? "a2b") !== "b2a";
  const amountIn = new BN(amountUi).mul(new BN(10).pow(new BN(DECIMALS)));

  console.log(`Направление: ${assetToPair ? "asset → pair" : "pair → asset"}`);
  const r = await vaultSwap(amountIn, assetToPair, slippageBps);
  console.log(`amount_in:      ${r.amountIn.toString()}`);
  console.log(`ожидаемо:       ${r.expectedOut.toString()}`);
  console.log(`min_amount_out: ${r.minAmountOut.toString()}  (slippage ${slippageBps / 100}%)`);
  console.log(`получено:       ${r.received}`);
  console.log("swap tx:", tx(r.sig));
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
