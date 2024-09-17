use anchor_lang::prelude::*;

#[error_code]
pub enum GameError {
    #[msg("The game has already started.")]
    GameAlreadyStarted,
    #[msg("The game is full.")]
    GameIsFull,
    #[msg("Player is already in the game.")]
    PlayerAlreadyInGame,
    #[msg("Not enough players to start the game.")]
    NotEnoughPlayers,
    #[msg("Player reached the maximum number of active games.")]
    TooManyActiveGames,
    #[msg("Invalid map size.")]
    InvalidMapSize,
    #[msg("The game is single player.")]
    GameIsSinglePlayer,
}
