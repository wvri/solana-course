const counterKp = web3.Keypair.generate();

const tx1 = await pg.program.methods
  .initialize()
  .accounts({
    counter: counterKp.publicKey,
    user: pg.wallet.publicKey,
    systemProgram: web3.SystemProgram.programId,
  })
  .signers([counterKp])
  .rpc();
await pg.connection.confirmTransaction(tx1, "confirmed");
console.log("initialize tx:", tx1);

const before = await pg.program.account.counter.fetch(counterKp.publicKey);
console.log("count before =", before.count.toString());

const tx2 = await pg.program.methods
  .increment()
  .accounts({
    counter: counterKp.publicKey,
    authority: pg.wallet.publicKey,
  })
  .rpc();
await pg.connection.confirmTransaction(tx2, "confirmed");
console.log("increment tx:", tx2);

const after = await pg.program.account.counter.fetch(counterKp.publicKey);
console.log("count after =", after.count.toString());
