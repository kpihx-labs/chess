use crate::cli::{Opponent, Options, Side};
use rand::random;
use std::time::{Duration, Instant};
use vendetta_chess_engine::{
    game::{Game as EngineGame, rules::GameResult},
    moves::generate_legal_moves,
    search::{SearchConfig, SearchEngine, SearchResult, elo_to_skill_level},
    utils::types::{Color, Move},
};

pub struct Game {
    pub inner: EngineGame,
    pub cursor: u8,
    pub selected: Option<u8>,
    pub moves: Vec<Move>,
    pub options: Options,
    pub human_color: Option<Color>,
    pub flipped: bool,
    pub last_search: Option<SearchResult>,
    pub timeout: Option<Color>,
    result: GameResult,
    engine: SearchEngine,
    remaining: [Duration; 2],
    clock_history: Vec<[Duration; 2]>,
    turn_started: Instant,
}

impl Game {
    pub fn new(options: Options) -> Self {
        let human_color = match options.opponent {
            Opponent::Human => None,
            Opponent::Computer => Some(match options.side {
                Side::White => Color::White,
                Side::Black => Color::Black,
                Side::Random => {
                    if random() {
                        Color::White
                    } else {
                        Color::Black
                    }
                }
            }),
        };
        let base = Duration::from_secs(options.minutes * 60);
        Self {
            inner: EngineGame::new(),
            cursor: if human_color == Some(Color::Black) {
                52
            } else {
                12
            },
            selected: None,
            moves: Vec::new(),
            options,
            human_color,
            flipped: human_color == Some(Color::Black),
            last_search: None,
            timeout: None,
            result: GameResult::Ongoing,
            engine: SearchEngine::new(),
            remaining: [base; 2],
            clock_history: Vec::new(),
            turn_started: Instant::now(),
        }
    }

    pub fn side_to_move(&self) -> Color {
        self.inner.board.side_to_move
    }

    pub fn remaining(&self, color: Color) -> Duration {
        let stored = self.remaining[color_index(color)];
        if color == self.side_to_move() && self.result() == GameResult::Ongoing {
            stored.saturating_sub(self.turn_started.elapsed())
        } else {
            stored
        }
    }

    pub fn result(&self) -> GameResult {
        self.result.clone()
    }

    pub fn is_human_turn(&self) -> bool {
        self.options.opponent == Opponent::Human || self.human_color == Some(self.side_to_move())
    }

    pub fn legal_moves(&self) -> Vec<Move> {
        generate_legal_moves(&mut self.inner.board.clone())
    }

    pub fn legal_targets(&self) -> Vec<u8> {
        self.selected
            .map(|from| {
                self.legal_moves()
                    .into_iter()
                    .filter(|mv| mv.from == from)
                    .map(|mv| mv.to)
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn select_or_move(&mut self) {
        if !self.is_human_turn() || self.result() != GameResult::Ongoing || self.timeout.is_some() {
            return;
        }
        if let Some(from) = self.selected {
            let mut candidates: Vec<_> = self
                .legal_moves()
                .into_iter()
                .filter(|mv| mv.from == from && mv.to == self.cursor)
                .collect();
            candidates.sort_by_key(|mv| std::cmp::Reverse(mv.promotion));
            if let Some(mv) = candidates.first().copied() {
                self.play(mv);
            } else {
                self.selected = None;
            }
        } else if self
            .inner
            .board
            .piece_at(self.cursor)
            .is_some_and(|(_, color)| color == self.side_to_move())
        {
            self.selected = Some(self.cursor);
        }
    }

    pub fn move_cursor(&mut self, dx: isize, dy: isize) {
        let file = (self.cursor % 8) as isize;
        let rank = (self.cursor / 8) as isize;
        self.cursor = ((rank + dy).clamp(0, 7) * 8 + (file + dx).clamp(0, 7)) as u8;
    }

    pub fn maybe_engine_move(&mut self) {
        if self.options.opponent != Opponent::Computer
            || self.is_human_turn()
            || self.result() != GameResult::Ongoing
            || self.timeout.is_some()
        {
            return;
        }
        let available = self
            .remaining(self.side_to_move())
            .as_millis()
            .min(u64::MAX as u128) as u64;
        let think = (available / 30).clamp(80, 2_500);
        let config = SearchConfig {
            movetime: Some(think),
            skill_level: elo_to_skill_level(self.options.elo),
            multipv: if self.options.elo < 1800 { 3 } else { 1 },
            ..SearchConfig::default()
        };
        let result = if config.multipv > 1 {
            let results = self.engine.search_multipv(&mut self.inner.board, &config);
            choose_limited(results, self.options.elo)
        } else {
            self.engine.search(&mut self.inner.board, &config)
        };
        let mv = result.best_move;
        self.last_search = Some(result);
        if !mv.is_null() {
            self.play(mv);
        }
    }

    pub fn update_timeout(&mut self) {
        let side = self.side_to_move();
        if self.remaining(side).is_zero() {
            self.timeout = Some(side);
        }
    }

    pub fn undo(&mut self) {
        let count = if self.options.opponent == Opponent::Computer {
            2
        } else {
            1
        };
        for _ in 0..count {
            if let Some(mv) = self.moves.pop() {
                self.inner.unmake_move(mv);
                if let Some(clocks) = self.clock_history.pop() {
                    self.remaining = clocks;
                }
            }
        }
        self.selected = None;
        self.timeout = None;
        self.result = self.inner.result();
        self.turn_started = Instant::now();
    }

    pub fn restart(&mut self) {
        *self = Self::new(self.options);
    }

    fn play(&mut self, mv: Move) {
        let mover = self.side_to_move();
        let elapsed = self.turn_started.elapsed();
        let index = color_index(mover);
        self.clock_history.push(self.remaining);
        self.remaining[index] = self.remaining[index].saturating_sub(elapsed)
            + Duration::from_secs(self.options.increment);
        self.inner.make_move(mv);
        self.moves.push(mv);
        self.result = self.inner.result();
        self.cursor = mv.to;
        self.selected = None;
        self.turn_started = Instant::now();
    }
}

fn choose_limited(mut results: Vec<SearchResult>, elo: u16) -> SearchResult {
    results.sort_by_key(|result| std::cmp::Reverse(result.score));
    let index = match elo {
        600..=999 => 2,
        1000..=1499 => 1,
        _ => 0,
    }
    .min(results.len().saturating_sub(1));
    results.swap_remove(index)
}

fn color_index(color: Color) -> usize {
    if color == Color::White { 0 } else { 1 }
}

pub fn format_clock(duration: Duration) -> String {
    let seconds = duration.as_secs();
    format!("{:02}:{:02}", seconds / 60, seconds % 60)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn options() -> Options {
        Options {
            opponent: Opponent::Human,
            elo: 1600,
            side: Side::White,
            minutes: 10,
            increment: 0,
        }
    }
    #[test]
    fn initial_position_has_twenty_moves() {
        assert_eq!(Game::new(options()).legal_moves().len(), 20);
    }
    #[test]
    fn cursor_move_is_bounded() {
        let mut game = Game::new(options());
        game.cursor = 0;
        game.move_cursor(-1, -1);
        assert_eq!(game.cursor, 0);
    }
    #[test]
    fn human_move_and_undo_round_trip() {
        let mut game = Game::new(options());
        game.cursor = 12;
        game.select_or_move();
        game.cursor = 28;
        game.select_or_move();
        assert_eq!(game.moves.len(), 1);
        game.undo();
        assert_eq!(game.inner.board.to_fen(), EngineGame::new().board.to_fen());
    }

    #[test]
    fn engine_answers_a_human_move() {
        let mut options = options();
        options.opponent = Opponent::Computer;
        options.elo = 600;
        let mut game = Game::new(options);
        game.cursor = 12;
        game.select_or_move();
        game.cursor = 28;
        game.select_or_move();
        game.maybe_engine_move();
        assert_eq!(game.moves.len(), 2);
        assert!(game.last_search.is_some());
    }
}
