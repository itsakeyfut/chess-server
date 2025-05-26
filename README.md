# Multiplayer Chess Server

This is a Rust server using Tokio and TCP, initially built for a chess game.
However, it is designed with extensibility in mind, and can be adapted to support various turn-based games beyond chess.

The game client is developed in Unity and communicates with this server over TCP.

## Features

- Asynchronous TCP server powered by Tokio
- Built-in chess game logic
- Extensible architecture for generic turn-based games
- Unity-compatible TCP protocol with simple message structure

## Planned Extensions

- Support for other turn-based games (e.g., Shogi, card games)
- Persistent session storage (e.g., Redis integration)
