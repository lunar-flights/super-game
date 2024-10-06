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

    const initialGameState = await program.account.game.fetch(gamePda);

    // successfully move unit from (1, 1) to (2, 1)
    // adjacent tile in the next row
    await program.methods
      .moveUnit(1, 1, 2, 1)
      .accounts({
        game: gamePda,
        player: player,
      })
      .rpc();

    const updatedGameState = await program.account.game.fetch(gamePda);
    timestamp = updatedGameState.turnTimestamp.toNumber();
    // some units died during attack on neutral tile
    expect(updatedGameState.tiles[2][1].units.quantity).to.equal(
      initialGameState.tiles[1][1].units.quantity - initialGameState.tiles[2][1].units.quantity
    );
    expect(updatedGameState.players[0].attackPoints).to.equal(initialGameState.players[0].attackPoints - 1);
    expect(updatedGameState.tiles[2][1].units.unitType).to.deep.equal({ infantry: {} });
    expect(updatedGameState.tiles[2][1].owner.toBase58()).to.be.equal(player.toBase58());
    expect(updatedGameState.tiles[2][1].units.stamina).to.equal(0);

    expect(updatedGameState.tiles[1][1].units).to.be.null;
  });

  it("Fails to recruit units in a tile that doesn't belong to player", async () => {
    const player = provider.wallet.publicKey;
    const gameData = { game_id: 0 };

    const [gamePda] = await anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("GAME"), new anchor.BN(gameData.game_id).toArrayLike(Buffer, "le", 4)],
      program.programId
    );
    const unitType = { infantry: {} };

    try {
      // tile (3, 3) doesn't belong to player
      await program.methods
        .recruitUnits(unitType, 1, 3, 3)
        .accounts({
          game: gamePda,
          player: player,
        })
        .rpc();
      throw new Error("Expected error, but transaction succeeded");
    } catch (error) {
      expect(error.error.errorCode.code).to.equal("TileNotOwned");
    }
  });

  it("Fails to recruit 100 infantry units in base tile due to insufficient funds", async () => {
    const player = provider.wallet.publicKey;
    const gameData = { game_id: 0 };

    const [gamePda] = await anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("GAME"), new anchor.BN(gameData.game_id).toArrayLike(Buffer, "le", 4)],
      program.programId
    );
    const unitType = { infantry: {} };

    try {
      // Not enough money to recruit 100 infantry units
      await program.methods
        .recruitUnits(unitType, 100, 1, 1)
        .accounts({
          game: gamePda,
          player: player,
        })
        .rpc();
      throw new Error("Expected error, but transaction succeeded");
    } catch (error) {
      expect(error.error.errorCode.code).to.equal("InsufficientFunds");
    }
  });

  it("Successfully recruits 2 infantry units in base tile", async () => {
    const player = provider.wallet.publicKey;
    const gameData = { game_id: 0 };

    const [gamePda] = await anchor.web3.PublicKey.findProgramAddressSync(
      [Buffer.from("GAME"), new anchor.BN(gameData.game_id).toArrayLike(Buffer, "le", 4)],
      program.programId
    );
    const unitType = { infantry: {} };

    const gameStateBefore = await program.account.game.fetch(gamePda);
    const playerInfoBefore = gameStateBefore.players[0];

    // Recruit 2 infantry units at tile (1, 1)
    await program.methods
      .recruitUnits(unitType, 2, 1, 1)
      .accounts({
        game: gamePda,
        player: player,
      })
      .rpc();

    const gameStateAfter = await program.account.game.fetch(gamePda);
    const playerInfoAfter = gameStateAfter.players[0];

    const unitCost = 1;
    const totalCost = unitCost * 2;
    expect(playerInfoAfter.balance).to.equal(playerInfoBefore.balance - totalCost);

    const tile = gameStateAfter.tiles[1][1];
    expect(tile.units.quantity).to.equal(2);
    expect(tile.units.unitType).to.deep.equal({ infantry: {} });
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
