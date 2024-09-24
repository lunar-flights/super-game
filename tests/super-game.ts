import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { expect } from "chai";
import { SuperGame } from "../target/types/super_game";

describe("super-game", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.SuperGame as Program<SuperGame>;

  let timestamp = 0;

  it("Initializes the program", async () => {
    const [superStatePda] = await anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("SUPER")],
      program.programId
    );

    await program.methods.initializeProgram().rpc();
    const superState = await program.account.superState.fetch(superStatePda);

    expect(superState.gameCount).to.equal(0);
  });

  it("Creates a player profile", async () => {
    const player = provider.wallet.publicKey;

    const [playerProfilePda] = await anchor.web3.PublicKey.findProgramAddressSync(
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

    const [superStatePda] = await anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("SUPER")],
      program.programId
    );

    const [playerProfilePda] = await anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("PROFILE"), player.toBuffer()],
      program.programId
    );

    const superState = await program.account.superState.fetch(superStatePda);
    let gameId = superState.gameCount;
    const [gamePda] = await anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("GAME"), new anchor.BN(gameId).toArrayLike(Buffer, "le", 4)],
      program.programId
    );

    await program.methods
      // max_players = 2, is_multiplayer = true, map_size = small
      .createGame(2, false, { small: {} })
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
    expect(game.status).deep.equal({ live: {} });
    expect(game.isMultiplayer).to.be.false;
    expect(game.mapSize).deep.equal({ small: {} });
    expect(game.tiles.length).to.equal(7);
    console.log(game);
    console.log(game.tiles[1]);
  });

  it("Fails to move a unit with 1 stamina to diagonal tile", async () => {
    const player = provider.wallet.publicKey;
    const gameData = { game_id: 0 };

    const [gamePda] = await anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("GAME"), new anchor.BN(gameData.game_id).toArrayLike(Buffer, "le", 4)],
      program.programId
    );

    try {
      // try to move unit with 1 stamina to diagonal tile (different row index and col index)
      // (1, 1) -> (2, 2)
      await program.methods
        .moveUnit(1, 1, 2, 2)
        .accounts({
          game: gamePda,
          player: player,
        })
        .rpc();
    } catch (error) {
      expect(error.error.errorCode.code).to.equal("NotEnoughStamina");
    }
  });

  it("Moves a unit from one tile to an adjacent tile", async () => {
    const player = provider.wallet.publicKey;
    const gameData = { game_id: 0 };

    const [gamePda] = await anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("GAME"), new anchor.BN(gameData.game_id).toArrayLike(Buffer, "le", 4)],
      program.programId
    );

    // successfully move unit from (1, 1) to (2, 1)
    // adjacent tile in the next row
    await program.methods
      .moveUnit(1, 1, 2, 1)
      .accounts({
        game: gamePda,
        player: player,
      })
      .rpc();

    const updatedGame = await program.account.game.fetch(gamePda);
    timestamp = updatedGame.turnTimestamp.toNumber();
    expect(updatedGame.tiles[2][1].units.quantity).to.equal(5);
    expect(updatedGame.tiles[2][1].units.unitType).to.deep.equal({ infantry: {} });
    // expect(updatedGame.tiles[2][1].owner.toBase58()).to.be.equal(player.toBase58());
    expect(updatedGame.tiles[2][1].units.stamina).to.equal(0);

    expect(updatedGame.tiles[1][1].units).to.be.null;
  });

  it("End turn", async () => {
    const player = provider.wallet.publicKey;
    const gameData = { game_id: 0 };
    const [gamePda] = await anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("GAME"), new anchor.BN(gameData.game_id).toArrayLike(Buffer, "le", 4)],
      program.programId
    );

    await program.methods
      .endTurn()
      .accounts({
        game: gamePda,
        player: player,
      })
      .rpc();

    const updatedGame = await program.account.game.fetch(gamePda);
    expect(updatedGame.currentPlayerIndex).to.equal(0);
    expect(updatedGame.turnTimestamp.toNumber()).to.be.greaterThan(timestamp);
    expect(updatedGame.round).to.equal(2);
    // restored stamina of unit who moved before
    expect(updatedGame.tiles[2][1].units.stamina).to.equal(1);
  });
});
