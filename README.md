# chess

`chess` is a responsive terminal chess client with complete rules, local 1v1, real clocks, and a strong native Rust engine. It aims for the clarity of a modern online chess board without accounts, browsers, services, or configuration overhead.

## Engine

The client uses Vendetta Chess Engine 1.1.2, a Rust bitboard engine with legal move generation, iterative deepening, alpha-beta/PVS search, quiescence, transposition tables, move ordering, tapered evaluation, draw detection, and native strength limiting.

Requested Elo spans 600–2600. Lower tiers combine Vendetta's internal Elo-to-skill mapping with MultiPV deliberate suboptimal selection; higher tiers play the principal move. The number is a target strength profile, not a certified tournament rating for this client build.

## Features

- Human vs engine and local human vs human.
- White, black, or random side selection.
- Configurable real chess clocks such as `10+0`, `5+3`, or `15+10`.
- Complete legal rules: castling, en passant, promotions, checks, mates, stalemate, repetition, fifty-move rule, and insufficient material.
- Responsive Unicode board, legal destination highlighting, last-move highlighting, board flip, and mouse selection.
- Live engine evaluation, depth, nodes, and search time in wide terminals.
- Move history, full-turn undo against the engine, and single-ply undo in local play.
- Frozen result board for checkmate, draws, and timeouts.

## Install

```bash
make install
```

## Play

```bash
chess
chess --elo 2200 --side black --time 15+10
chess --side random --elo 900 --time 5+3
chess --vs human --time 10+0
```

| Action | Keyboard / mouse |
|---|---|
| Move cursor | Arrows or `hjkl` |
| Select / move | `Space`, `Enter`, or left click |
| Cancel selection | `Esc` |
| Flip board | `F` |
| Undo | `U` |
| Full move history | `M` |
| New game | `N` or `R` |
| Help / quit | `?` / `Q` |

Promotions default to queen, the strongest and overwhelmingly standard choice. All four promotion types remain supported by the rules engine and can be exposed in a later explicit promotion picker without changing the game core.

## References

Ignored read-only research clones live under `refs/`:

- `Rust-Vendetta-Chess-Engine`: engine dependency and strength-limiting reference.
- `chess-tui`: terminal interaction and board UX precedent.
- `cozy-chess`: fast Rust legal-move architecture reference.

The client code is original. Vendetta remains an MIT-licensed dependency with its own copyright and license.

## Quality

```bash
make check
```

The gate runs formatting, strict Clippy, all tests including a real engine response, a locked release build, and CLI smoke tests.

## License

Business Source License 1.1 for this client. Personal, noncommercial, teaching, and academic production use is granted; it converts to `AGPL-3.0-only` on 2030-11-05. Dependencies retain their own licenses.
