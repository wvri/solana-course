const [profilePda] = web3.PublicKey.findProgramAddressSync(
  [Buffer.from("profile"), pg.wallet.publicKey.toBuffer()],
  pg.program.programId
);
console.log("Profile PDA:", profilePda.toBase58());

const createTx = await pg.program.methods
  .createProfile("Alice", "Solana developer")
  .accounts({
    profile: profilePda,
    user: pg.wallet.publicKey,
    systemProgram: web3.SystemProgram.programId,
  })
  .rpc();
await pg.connection.confirmTransaction(createTx, "confirmed");
console.log("create tx:", createTx);

const before = await pg.program.account.profile.fetch(profilePda);
console.log("before:", `${before.name} | ${before.bio}`);

const updateTx = await pg.program.methods
  .updateProfile("Alice Updated", "Learning Anchor and PDAs")
  .accounts({
    profile: profilePda,
    authority: pg.wallet.publicKey,
  })
  .rpc();
await pg.connection.confirmTransaction(updateTx, "confirmed");
console.log("update tx:", updateTx);

const after = await pg.program.account.profile.fetch(profilePda);
console.log("after:", `${after.name} | ${after.bio}`);
