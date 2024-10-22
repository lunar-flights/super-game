### SUPER: chess meets StarCraft.

Play: https://super.game

> SUPER is a fully on-chain strategy game on Solana, developed by Lunar Flights.

This is a on-chain program repository. The web frontend can be found here: https://github.com/lunar-flights/super-game-frontend

#### Run locally
```
anchor build && anchor test
```

<img width="600" alt="super-screenshot" src="https://github.com/user-attachments/assets/ea2a3683-719d-4aaf-90c3-1b4e3c1dd5cd">

## Gameplay overview

SUPER is a competitive sci-fi strategy game, a hybrid of RTS and turn-based game, where players build units, capture tiles, and battle each other to achieve dominance. There are two ways to win:
1. Control 51% or more of the tiles on the map.
2. Be the last player standing by destroying all other players capitals.

When a player (or bot) is eliminated, their base tile is captured by the player who destroyed it, while all their other tiles are removed from the game, creating "holes" in the grid.

## Units

Each unit type has different costs, stamina, and strength. Mutants are neutral NPC units.

| Unit Type  | Cost | Stamina | Strength | Description                               |
|------------|------|---------|----------|-------------------------------------------|
| Infantry   | 1    | 1       | 1        | Basic unit that can be acquired on any tile controlled by a player.|
| Tank       | 3    | 3       | 3        | Advanced unit that can be purchased only in Tank Factory. Can attack diagonal tiles and move after attacks. |
| Plane      | 5    | 5       | 4        | Advanced unit that can be purchased only in Plane Factory. Can attack diagonal tiles and move after attacks. |
| Mutants    | 0    | 0       | 1        | Neutral units, same strength as infantry, cannot move.|

## Buildings

Buildings can either produce resources per turn or unlock advanced units for production.

| Building Type     | Level | Yield per Turn | Unlocks / Description                   | Strength |
|-------------------|-------|----------------|-----------------------------------------|----------|
| Capital              | 1     | 3              | Players are eliminated if their Capital is destroyed.     | 12       |
| Capital              | 2     | 4              |                                         | 16       |
| Capital              | 3     | 6              |                                         | 24       |
| Gas Plant         | -     | 1              | Generates extra resources per turn.     | -        |
| Tank Factory      | -     | 0              | Unlocks the ability to produce tanks.    | -        |
| Plane Factory     | -     | 0              | Unlocks the ability to produce planes.   | -        |
| Fort              | -     | 0              | Increases defense strength of a tile.    | 7        |

## Tile Types

| Tile Level | Yield | Defense Bonus (Non-Mutants) |
|------------|-------|-----------------------------|
| 1          | 0     | 1                           |
| 2          | 0     | 2                           |
| 3          | 1     | 3                           |

- **Neutral Tiles**: These tiles are occupied by mutants but offer no defense bonuses to them.
- **Defense Bonus**: The defense bonus applies to any players troops positioned in a tile.
