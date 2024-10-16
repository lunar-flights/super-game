use anchor_lang::prelude::*;

#[error_code]
pub enum GameError {
    #[msg("The game has already started")]
    GameAlreadyStarted,
    #[msg("The game is full")]
    GameIsFull,
    #[msg("Player is already in the game")]
    PlayerAlreadyInGame,
    #[msg("Not enough players to start the game")]
    NotEnoughPlayers,
    #[msg("Player reached the maximum number of active games")]
    TooManyActiveGames,
    #[msg("Invalid map size")]
    InvalidMapSize,
    #[msg("The game is single player")]
    GameIsSinglePlayer,
    #[msg("Invalid player")]
    InvalidPlayer,
    #[msg("Not your turn")]
    NotYourTurn,
    #[msg("Destination is out of map bounds")]
    OutOfBounds,
    #[msg("Bot key not found")]
    BotKeyNotFound,
    #[msg("Invalid coordinates")]
    InvalidCoordinates,
    #[msg("Tile does not exist")]
    InvalidTile,
    #[msg("You do not own this tile")]
    TileNotOwned,
    #[msg("Different unit type already present on the tile")]
    DifferentUnitTypeOnTile,
    #[msg("A Tank Factory is required to recruit tanks")]
    RequiresTankFactory,
    #[msg("A Plane Factory is required to recruit planes")]
    RequiresPlaneFactory,
    #[msg("Invalid unit type")]
    InvalidUnitType,
    #[msg("Insufficient funds")]
    InsufficientFunds,
    #[msg("Too many units")]
    TooManyUnits,
}

#[error_code]
pub enum UnitError {
    #[msg("This is not your tile")]
    NotYourTile,
    #[msg("No units to move")]
    NoUnitsToMove,
    #[msg("Destination tile is not adjacent")]
    InvalidMovement,
    #[msg("Not enough stamina")]
    NotEnoughStamina,
    #[msg("Invalid tile")]
    InvalidTile,
    #[msg("Tile is occupied by other unit")]
    TileOccupiedByOtherUnitType,
    #[msg("Tile is occupied by enemy unit")]
    TileOccupiedByEnemy,
    #[msg("No attack points")]
    NotEnoughAttackPoints,
}

#[error_code]
pub enum ConstructionError {
    #[msg("You do not own this tile")]
    NotYourTile,
    #[msg("Not enough funds to construct or upgrade")]
    NotEnoughFunds,
    #[msg("Tile already has a different building")]
    BuildingTypeMismatch,
    #[msg("Building has reached its maximum level")]
    MaxLevelReached,
    #[msg("Cannot build a Base")]
    CannotBuildBase,
}
