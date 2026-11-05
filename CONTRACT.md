# Runtime Contract

## Rules

- Every accepted move comes from Vendetta's legal move generator.
- Checkmate, stalemate, repetition, fifty-move, and insufficient-material results come from the engine game state.
- Promotions choose queen when multiple legal promotions share the selected destination.
- A timeout freezes the position and identifies the side that lost on time.

## Engine

- Target Elo is validated within 600–2600.
- Search uses Vendetta's Elo-to-skill mapping and a clock-derived bounded move time.
- Lower levels deliberately select lower-ranked MultiPV lines; higher levels select the principal line.
- UI telemetry reports the actual returned score, depth, nodes, and elapsed search time.

## Interface

- The complete board always remains visible; cells expand with terminal geometry.
- Wide terminals add clocks, history, and engine telemetry without shrinking the board.
- Local play undoes one ply; engine play undoes a complete human-engine turn.
- Undo restores board state and both clocks.
- Terminal state and mouse capture are restored on normal exit.
