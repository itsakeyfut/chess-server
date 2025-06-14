use chess_server::{
    game::{Board, GameManager, MoveValidator, Position, Move},
    network::ChessServer,
    utils::{load_config, ServerConfig},
};
use tokio::signal;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Chess Server Starting...");

    let config = match load_config() {
        Ok(config) => {
            println!("Configuration loaded successfully");
            config
        }
        Err(e) => {
            eprintln!("Failed to load configuration: {}", e);
            println!("Using default configuration");
            ServerConfig::development()
        }
    };

    println!("Server configuration:");
    println!("  Host: {}", config.server.host);
    println!("  Port: {}", config.server.port);
    println!("  Max connections: {}", config.server.max_connections);
    println!("  Log level: {}", config.logging.level);

    test_chess_logic();

    let server = ChessServer::new(config);

    let server_for_shutdown = std::sync::Arc::new(server);
    let server_for_run = server_for_shutdown.clone();

    let shutdown_task = tokio::spawn(async move {
        signal::ctrl_c().await.expect("Failed to install Ctrl+C handler");
        println!("\nRecived Ctrl+C, shutting down...");
        server_for_shutdown.stop().await;
    });

    let server_task = tokio::spawn(async move {
        if let Err(e) = server_for_run.start().await {
            eprintln!("Server error: {}", e);
        }
    });

    tokio::select! {
        _ = server_task => {
            println!("Server task completed");
        }
        _ = shutdown_task => {
            println!("Shutdown task completed");
        }
    }

    println!("Chess Server stopped");
    Ok(())
}

fn test_chess_logic() {
    println!("Testing chess logic...");

    let board = Board::new();
    println!("Initial board:");
    println!("{}", board.display());
    println!("FEN: {}", board.to_fen());

    let legal_moves = MoveValidator::generate_legal_moves(&board);
    println!("Legal moves from starting position: {}", legal_moves.len());

    for (i, chess_move) in legal_moves.iter().take(5).enumerate() {
        println!("  {}. {}", i + 1, chess_move.to_algebraic());
    }

    let mut game_manager = GameManager::new();
    let game_id = game_manager.create_game();
    println!("Created game: {}", game_id);

    match game_manager.join_game(&game_id, "itsakeyfut".to_string(), None) {
        Ok(color) => println!("itsakeyfut joined as {:?}", color),
        Err(e) => println!("Failed to add itsakeyfut: {}", e),
    }

    match game_manager.join_game(&game_id, "dullboy".to_string(), None) {
        Ok(color) => println!("dullboy joined as {:?}", color),
        Err(e) => println!("Failed to add dullboy: {}", e),
    }

    if let Some(first_move) = legal_moves.first() {
        match game_manager.make_move(&game_id, "Alice", first_move.clone()) {
            Ok(()) => println!("Move made: {}", first_move.to_algebraic()),
            Err(e) => println!("Failed to make move: {}", e),
        }
    }
    
    if let Some(game) = game_manager.get_game(&game_id) {
        println!("Game after first move:");
        println!("{}", game.board.display());
        println!("Turn: {:?}", game.board.get_to_move());
        println!("Move count: {}", game.get_move_count());
    }
    
    println!("Chess logic test completed!");
}