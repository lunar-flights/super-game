import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { expect } from "chai";
import { SuperGame } from "../target/types/super_game";

describe("super-game", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.SuperGame as Program<SuperGame>;

  it("Initializes the program", async () => {
    const [superStatePda] = await anchor.web3.PublicKey.findProgramAddress([Buffer.from("SUPER")], program.programId);

    await program.methods.initializeProgram().rpc();
    const superState = await program.account.superState.fetch(superStatePda);

    expect(superState.gameCount).to.equal(0);
  });

  it("Creates a player profile", async () => {
    const player = provider.wallet.publicKey;

    const [playerProfilePda] = await anchor.web3.PublicKey.findProgramAddress(
      [Buffer.from("PROFILE"), player.toBuffer()],
      program.programId
    );

    await program.methods
      .createPlayerProfile()
      .accounts({
        player: player,
      })
      .rpc();

    const playerProfile = await program.account.playerProfile.fetch(playerProfilePda);
    expect(playerProfile.player.toBase58()).to.be.equal(player.toBase58());
    expect(playerProfile.completedGames).to.be.equal(0);
  });

  it("Creates a game", async () => {
    const player = provider.wallet.publicKey;

    const [superStatePda] = await anchor.web3.PublicKey.findProgramAddress([Buffer.from("SUPER")], program.programId);

    const [playerProfilePda] = await anchor.web3.PublicKey.findProgramAddress(
      [Buffer.from("PROFILE"), player.toBuffer()],
      program.programId
    );

    const superState = await program.account.superState.fetch(superStatePda);
    let gameId = superState.gameCount;
    const [gamePda] = await anchor.web3.PublicKey.findProgramAddress(
      [Buffer.from("GAME"), new anchor.BN(gameId).toArrayLike(Buffer, "le", 4)],
      program.programId
    );

    await program.methods
      // max_players = 2, is_multiplayer = true, map_size = small
      .createGame(2, true, { small: {} })
      .accounts({
        superState: superStatePda,
        game: gamePda,
        creator: player,
      })
      .rpc();

    const newSuperState = await program.account.superState.fetch(superStatePda);
    const game = await program.account.game.fetch(gamePda);
    expect(newSuperState.gameCount).to.equal(gameId + 1);
    expect(game.creator.toBase58()).to.be.equal(player.toBase58());
    expect(game.status).deep.equal({ notStarted: {} });
    expect(game.isMultiplayer).to.be.true;
    expect(game.mapSize).deep.equal({ small: {} });
    expect(game.tiles.length).to.equal(37);
    console.log(game);
    console.log(game.tiles[1]);
  });
});
