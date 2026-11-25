/**
 * Общие настройки devnet-скриптов (недели 11–13).
 *
 * Кошелёк: ANCHOR_WALLET или ~/.config/solana/id.json
 * RPC:     RPC_URL или https://api.devnet.solana.com
 * Адреса созданных токенов/пула сохраняются в scripts/state.json (в .gitignore).
 */
import * as anchor from "@anchor-lang/core";
import { DEV_API_URLS, DEVNET_PROGRAM_ID, Raydium, TxVersion } from "@raydium-io/raydium-sdk-v2";
import { Connection, Keypair, PublicKey, clusterApiUrl } from "@solana/web3.js";
import fs from "fs";
import os from "os";
import path from "path";
import type { LiquidityVault } from "../target/types/liquidity_vault";

export const RPC_URL = process.env.RPC_URL ?? clusterApiUrl("devnet");
export const connection = new Connection(RPC_URL, "confirmed");

const walletPath = process.env.ANCHOR_WALLET ?? path.join(os.homedir(), ".config/solana/id.json");
export const owner = Keypair.fromSecretKey(
  Uint8Array.from(JSON.parse(fs.readFileSync(walletPath, "utf8")))
);

export const txVersion = TxVersion.V0;
/** Raydium CLMM в devnet: DRayAUgENGQBKVaX8owNhgzkEDyoHTGVEGHVJT1E9pfH */
export const CLMM_PROGRAM_ID: PublicKey = DEVNET_PROGRAM_ID.CLMM_PROGRAM_ID;
export const MEMO_PROGRAM_ID = new PublicKey("MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr");

export const tx = (sig: string) => `https://explorer.solana.com/tx/${sig}?cluster=devnet`;
export const addr = (a: PublicKey | string) =>
  `https://explorer.solana.com/address/${a.toString()}?cluster=devnet`;

// ───────────── state.json ─────────────

export type State = Partial<{
  mintA: string;
  mintB: string;
  poolId: string;
  vaultConfig: string;
  positionNftMint: string;
}>;

const statePath = path.join(__dirname, "state.json");

export function loadState(): State {
  return fs.existsSync(statePath) ? JSON.parse(fs.readFileSync(statePath, "utf8")) : {};
}

export function saveState(patch: State): State {
  const next = { ...loadState(), ...patch };
  fs.writeFileSync(statePath, JSON.stringify(next, null, 2));
  return next;
}

export function need<K extends keyof State>(state: State, key: K): string {
  const v = state[key];
  if (!v) throw new Error(`В scripts/state.json нет "${key}" — запустите предыдущий скрипт`);
  return v;
}

// ───────────── Raydium SDK ─────────────

let raydium: Raydium | undefined;

export async function initSdk(): Promise<Raydium> {
  if (raydium) return raydium;
  raydium = await Raydium.load({
    owner,
    connection,
    cluster: "devnet",
    disableFeatureCheck: true,
    disableLoadToken: true,
    blockhashCommitment: "finalized",
    urlConfigs: {
      ...DEV_API_URLS,
      BASE_HOST: "https://api-v3-devnet.raydium.io",
      OWNER_BASE_HOST: "https://owner-v1-devnet.raydium.io",
      SWAP_HOST: "https://transaction-v1-devnet.raydium.io",
      CPMM_LOCK: "https://dynamic-ipfs-devnet.raydium.io/lock/cpmm/position",
    },
  });
  return raydium;
}

// ───────────── Программа vault ─────────────

export function vaultProgram(): anchor.Program<LiquidityVault> {
  const provider = new anchor.AnchorProvider(connection, new anchor.Wallet(owner), {
    commitment: "confirmed",
  });
  anchor.setProvider(provider);
  const idl = require("../target/idl/liquidity_vault.json");
  return new anchor.Program<LiquidityVault>(idl, provider);
}

export function vaultPdas(programId: PublicKey, assetMint: PublicKey) {
  const pda = (...seeds: Buffer[]) => PublicKey.findProgramAddressSync(seeds, programId)[0];
  const config = pda(Buffer.from("config"), assetMint.toBuffer());
  return {
    config,
    vaultAuthority: pda(Buffer.from("authority"), config.toBuffer()),
    shareMint: pda(Buffer.from("shares"), config.toBuffer()),
    vaultAsset: pda(Buffer.from("vault_asset"), config.toBuffer()),
    strategy: pda(Buffer.from("strategy"), config.toBuffer()),
    vaultPair: pda(Buffer.from("vault_pair"), config.toBuffer()),
    userAccount: (user: PublicKey) => pda(Buffer.from("user"), config.toBuffer(), user.toBuffer()),
  };
}
