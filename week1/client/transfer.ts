const recipient = web3.Keypair.generate().publicKey;

const tx = new web3.Transaction().add(
  web3.SystemProgram.transfer({
    fromPubkey: pg.wallet.publicKey,
    toPubkey: recipient,
    lamports: 0.1 * web3.LAMPORTS_PER_SOL,
  })
);

const signature = await web3.sendAndConfirmTransaction(
  pg.connection,
  tx,
  [pg.wallet.keypair]
);

console.log("Recipient:", recipient.toBase58());
console.log("Signature:", signature);
console.log(`https://explorer.solana.com/tx/${signature}?cluster=devnet`);
