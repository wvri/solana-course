/**
 * Неделя 11, шаг 1: создаём два тестовых SPL-токена в devnet
 * и выпускаем по 1 000 000 каждого на свой кошелёк.
 *
 *   yarn ts-node scripts/01-create-tokens.ts
 */
import { createMint, getOrCreateAssociatedTokenAccount, mintTo } from "@solana/spl-token";
import { addr, connection, owner, saveState, tx } from "./common";

const DECIMALS = 6;
const SUPPLY = 1_000_000n * 10n ** BigInt(DECIMALS);

async function createTestToken(label: string) {
  const mint = await createMint(connection, owner, owner.publicKey, null, DECIMALS);
  const ata = await getOrCreateAssociatedTokenAccount(connection, owner, mint, owner.publicKey);
  const sig = await mintTo(connection, owner, mint, ata.address, owner, SUPPLY);
  console.log(`${label}: ${mint.toBase58()}`);
  console.log(`  mint:    ${addr(mint)}`);
  console.log(`  mintTo:  ${tx(sig)}`);
  return mint;
}

async function main() {
  console.log("Wallet:", owner.publicKey.toBase58());
  const mintA = await createTestToken("Token A (актив vault)");
  const mintB = await createTestToken("Token B (вторая сторона пары)");
  saveState({ mintA: mintA.toBase58(), mintB: mintB.toBase58() });
  console.log("Сохранено в scripts/state.json");
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
