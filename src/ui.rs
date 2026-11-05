use crate::{
    cli::{Opponent, Options},
    game::{Game, format_clock},
};
use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, MouseButton, MouseEventKind,
    },
    execute, queue,
    style::{
        Attribute, Color as TermColor, Print, ResetColor, SetAttribute, SetBackgroundColor,
        SetForegroundColor,
    },
    terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{
    io::{Stdout, Write, stdout},
    time::Duration,
};
use vendetta_chess_engine::{
    game::rules::GameResult,
    moves::is_in_check,
    utils::types::{Color, Piece},
};

pub struct App {
    game: Game,
    modal: Modal,
    geometry: Geometry,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Modal {
    None,
    Help,
    History,
}
#[derive(Debug, Clone, Copy, Default)]
struct Geometry {
    x: u16,
    y: u16,
    cell_width: u16,
    cell_height: u16,
    panel_x: Option<u16>,
}
struct TerminalGuard;

impl TerminalGuard {
    fn enter(stdout: &mut Stdout) -> std::io::Result<Self> {
        terminal::enable_raw_mode()?;
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture, Hide)?;
        Ok(Self)
    }
}
impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let mut stdout = stdout();
        let _ = execute!(
            stdout,
            Show,
            DisableMouseCapture,
            LeaveAlternateScreen,
            ResetColor
        );
        let _ = terminal::disable_raw_mode();
    }
}

impl App {
    pub fn new(options: Options) -> Self {
        Self {
            game: Game::new(options),
            modal: Modal::None,
            geometry: Geometry::default(),
        }
    }

    pub fn run(mut self) -> std::io::Result<()> {
        let mut stdout = stdout();
        let _guard = TerminalGuard::enter(&mut stdout)?;
        loop {
            self.game.update_timeout();
            self.game.maybe_engine_move();
            self.draw(&mut stdout)?;
            if event::poll(Duration::from_millis(100))? && !self.handle(event::read()?) {
                break;
            }
        }
        Ok(())
    }

    fn handle(&mut self, event: Event) -> bool {
        if self.modal != Modal::None {
            if let Event::Key(key) = event {
                match key.code {
                    KeyCode::Esc | KeyCode::Char('q') => self.modal = Modal::None,
                    KeyCode::Char('?') if self.modal == Modal::Help => self.modal = Modal::None,
                    KeyCode::Char('m') if self.modal == Modal::History => self.modal = Modal::None,
                    _ => {}
                }
            }
            return true;
        }
        match event {
            Event::Key(key) => match key.code {
                KeyCode::Char('q') => return false,
                KeyCode::Esc => self.game.selected = None,
                KeyCode::Char('?') => self.modal = Modal::Help,
                KeyCode::Char('m') => self.modal = Modal::History,
                KeyCode::Char('f') => self.game.flipped = !self.game.flipped,
                KeyCode::Char('u') => self.game.undo(),
                KeyCode::Char('r') | KeyCode::Char('n') => self.game.restart(),
                KeyCode::Up | KeyCode::Char('k') => self.visual_move(0, -1),
                KeyCode::Down | KeyCode::Char('j') => self.visual_move(0, 1),
                KeyCode::Left | KeyCode::Char('h') => self.visual_move(-1, 0),
                KeyCode::Right | KeyCode::Char('l') => self.visual_move(1, 0),
                KeyCode::Enter | KeyCode::Char(' ') => self.game.select_or_move(),
                _ => {}
            },
            Event::Mouse(mouse)
                if matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) =>
            {
                if let Some(square) = self.mouse_square(mouse.column, mouse.row) {
                    self.game.cursor = square;
                    self.game.select_or_move();
                }
            }
            _ => {}
        }
        true
    }

    fn visual_move(&mut self, dx: isize, dy: isize) {
        let (dx, dy) = if self.game.flipped {
            (-dx, dy)
        } else {
            (dx, -dy)
        };
        self.game.move_cursor(dx, dy);
    }

    fn draw(&mut self, stdout: &mut Stdout) -> std::io::Result<()> {
        let (width, height) = terminal::size()?;
        self.geometry = geometry(width, height);
        queue!(stdout, Clear(ClearType::All))?;
        self.draw_header(stdout, width)?;
        self.draw_board(stdout)?;
        self.draw_panel(stdout, width)?;
        self.draw_footer(stdout, width, height)?;
        match self.modal {
            Modal::Help => self.draw_help(stdout, width, height)?,
            Modal::History => self.draw_history(stdout, width, height)?,
            Modal::None => {
                if let Some(message) = self.result_message() {
                    self.draw_result(stdout, width, height, &message)?;
                }
            }
        }
        stdout.flush()
    }

    fn draw_header(&self, stdout: &mut Stdout, width: u16) -> std::io::Result<()> {
        let side = if self.game.side_to_move() == Color::White {
            "WHITE"
        } else {
            "BLACK"
        };
        let opponent = if self.game.options.opponent == Opponent::Human {
            "LOCAL 1v1".to_string()
        } else {
            format!("ENGINE {} Elo", self.game.options.elo)
        };
        let check = if is_in_check(&self.game.inner.board, self.game.side_to_move()) {
            " · CHECK"
        } else {
            ""
        };
        let text = format!(" CHESS · {opponent} · {side} TO MOVE{check} ");
        queue!(
            stdout,
            MoveTo(0, 0),
            SetBackgroundColor(TermColor::DarkBlue),
            SetForegroundColor(TermColor::White),
            Print(center(&text, width as usize)),
            ResetColor
        )
    }

    fn draw_board(&self, stdout: &mut Stdout) -> std::io::Result<()> {
        let legal_targets = self.game.legal_targets();
        for visual_rank in 0..8 {
            for visual_file in 0..8 {
                let square = self.visual_square(visual_file, visual_rank);
                let x = self.geometry.x + visual_file * self.geometry.cell_width;
                let y = self.geometry.y + visual_rank * self.geometry.cell_height;
                let file = square % 8;
                let rank = square / 8;
                let light = (file + rank) % 2 == 1;
                let last = self
                    .game
                    .moves
                    .last()
                    .is_some_and(|mv| mv.from == square || mv.to == square);
                let target = legal_targets.contains(&square);
                let background = if square == self.game.cursor {
                    TermColor::DarkCyan
                } else if self.game.selected == Some(square) {
                    TermColor::DarkYellow
                } else if target {
                    TermColor::DarkGreen
                } else if last {
                    TermColor::DarkMagenta
                } else if light {
                    TermColor::Rgb {
                        r: 181,
                        g: 136,
                        b: 99,
                    }
                } else {
                    TermColor::Rgb {
                        r: 92,
                        g: 64,
                        b: 51,
                    }
                };
                let symbol = self
                    .game
                    .inner
                    .board
                    .piece_at(square)
                    .map(|(piece, color)| piece_symbol(piece, color))
                    .unwrap_or(" ");
                for inner in 0..self.geometry.cell_height {
                    let content = if inner == self.geometry.cell_height / 2 {
                        center(symbol, self.geometry.cell_width as usize)
                    } else {
                        " ".repeat(self.geometry.cell_width as usize)
                    };
                    queue!(
                        stdout,
                        MoveTo(x, y + inner),
                        SetBackgroundColor(background),
                        SetForegroundColor(TermColor::White),
                        SetAttribute(Attribute::Bold),
                        Print(content),
                        ResetColor,
                        SetAttribute(Attribute::Reset)
                    )?;
                }
            }
        }
        self.draw_coordinates(stdout)
    }

    fn draw_coordinates(&self, stdout: &mut Stdout) -> std::io::Result<()> {
        for visual in 0..8 {
            let square = self.visual_square(visual, 7);
            let file = char::from(b'a' + square % 8);
            queue!(
                stdout,
                MoveTo(
                    self.geometry.x + visual * self.geometry.cell_width,
                    self.geometry.y + 8 * self.geometry.cell_height
                ),
                SetForegroundColor(TermColor::DarkGrey),
                Print(file)
            )?;
            let square = self.visual_square(0, visual);
            let rank = char::from(b'1' + square / 8);
            if self.geometry.x > 0 {
                queue!(
                    stdout,
                    MoveTo(
                        self.geometry.x - 1,
                        self.geometry.y
                            + visual * self.geometry.cell_height
                            + self.geometry.cell_height / 2
                    ),
                    Print(rank)
                )?;
            }
        }
        Ok(())
    }

    fn draw_panel(&self, stdout: &mut Stdout, width: u16) -> std::io::Result<()> {
        let Some(x) = self.geometry.panel_x else {
            return Ok(());
        };
        let white = format_clock(self.game.remaining(Color::White));
        let black = format_clock(self.game.remaining(Color::Black));
        let lines = [
            format!("BLACK  {black}"),
            format!("WHITE  {white}"),
            String::new(),
            "MOVES".into(),
        ];
        for (index, line) in lines.iter().enumerate() {
            queue!(
                stdout,
                MoveTo(x, self.geometry.y + index as u16),
                SetForegroundColor(TermColor::White),
                Print(truncate(line, width.saturating_sub(x) as usize))
            )?;
        }
        let start = self.game.moves.len().saturating_sub(18);
        for (offset, pair) in self.game.moves[start..].chunks(2).enumerate() {
            let number = (start / 2) + offset + 1;
            let white = pair.first().map(|mv| mv.to_uci()).unwrap_or_default();
            let black = pair.get(1).map(|mv| mv.to_uci()).unwrap_or_default();
            queue!(
                stdout,
                MoveTo(x, self.geometry.y + 4 + offset as u16),
                SetForegroundColor(TermColor::Grey),
                Print(truncate(
                    &format!("{number:>2}. {white:<6} {black}"),
                    width.saturating_sub(x) as usize
                ))
            )?;
        }
        if let Some(search) = &self.game.last_search {
            let y = self.geometry.y + self.geometry.cell_height * 8 - 3;
            queue!(
                stdout,
                MoveTo(x, y),
                SetForegroundColor(TermColor::Cyan),
                Print(format!("eval {:+.2}", search.score as f64 / 100.0)),
                MoveTo(x, y + 1),
                Print(format!(
                    "depth {} · {}k nodes",
                    search.depth,
                    search.nodes / 1000
                )),
                MoveTo(x, y + 2),
                Print(format!("{} ms", search.time_ms))
            )?;
        }
        Ok(())
    }

    fn draw_footer(&self, stdout: &mut Stdout, width: u16, height: u16) -> std::io::Result<()> {
        let text =
            " arrows/hjkl move  Space select/move  F flip  U undo  M moves  ? help  N new  Q quit ";
        queue!(
            stdout,
            MoveTo(0, height.saturating_sub(1)),
            SetForegroundColor(TermColor::DarkGrey),
            Print(center(text, width as usize)),
            ResetColor
        )
    }

    fn draw_help(&self, stdout: &mut Stdout, width: u16, height: u16) -> std::io::Result<()> {
        let lines = vec![
            " CHESS HELP ".into(),
            "Select a piece, then a green legal destination.".into(),
            "Promotions default to queen; castling and en passant are fully legal.".into(),
            "F flips the board. U takes back one local ply or a full engine turn.".into(),
            "Purple marks the last move; cyan is the cursor; yellow is selection.".into(),
            "Esc or ? closes help.".into(),
        ];
        overlay(stdout, width, height, &lines, TermColor::Cyan)
    }

    fn draw_history(&self, stdout: &mut Stdout, width: u16, height: u16) -> std::io::Result<()> {
        let mut lines = vec![" MOVE HISTORY · UCI ".into()];
        lines.extend(self.game.moves.chunks(2).enumerate().map(|(index, pair)| {
            format!(
                "{:>2}. {:<6} {}",
                index + 1,
                pair[0].to_uci(),
                pair.get(1).map(|mv| mv.to_uci()).unwrap_or_default()
            )
        }));
        lines.push("Esc or M closes history.".into());
        overlay(stdout, width, height, &lines, TermColor::Yellow)
    }

    fn draw_result(
        &self,
        stdout: &mut Stdout,
        width: u16,
        height: u16,
        result: &str,
    ) -> std::io::Result<()> {
        let lines = vec![
            format!(" {result} "),
            format!("{} moves played", self.game.moves.len()),
            "N new game   M history   Q quit".into(),
        ];
        overlay(stdout, width, height, &lines, TermColor::Green)
    }

    fn result_message(&self) -> Option<String> {
        if let Some(loser) = self.game.timeout {
            return Some(format!("{} loses on time", color_name(loser)));
        }
        match self.game.result() {
            GameResult::Ongoing => None,
            GameResult::Checkmate => Some(format!(
                "Checkmate · {} wins",
                color_name(opposite(self.game.side_to_move()))
            )),
            GameResult::DrawFiftyMoves => Some("Draw · fifty-move rule".into()),
            GameResult::DrawRepetition => Some("Draw · threefold repetition".into()),
            GameResult::DrawInsufficientMaterial => Some("Draw · insufficient material".into()),
            GameResult::DrawStalemate => Some("Draw · stalemate".into()),
        }
    }

    fn visual_square(&self, file: u16, rank: u16) -> u8 {
        if self.game.flipped {
            (rank * 8 + (7 - file)) as u8
        } else {
            ((7 - rank) * 8 + file) as u8
        }
    }

    fn mouse_square(&self, column: u16, row: u16) -> Option<u8> {
        if column < self.geometry.x || row < self.geometry.y {
            return None;
        }
        let file = (column - self.geometry.x) / self.geometry.cell_width;
        let rank = (row - self.geometry.y) / self.geometry.cell_height;
        (file < 8 && rank < 8).then(|| self.visual_square(file, rank))
    }
}

fn geometry(width: u16, height: u16) -> Geometry {
    let available_height = height.saturating_sub(4);
    let cell_height = (available_height / 8).clamp(1, 4);
    let board_only_width = width.min(88);
    let cell_width = (board_only_width / 8).clamp(3, 9);
    let board_width = cell_width * 8;
    let board_height = cell_height * 8;
    let panel = width >= board_width + 28;
    let total_width = board_width + if panel { 28 } else { 0 };
    let x = width.saturating_sub(total_width) / 2;
    Geometry {
        x,
        y: 1 + available_height.saturating_sub(board_height) / 2,
        cell_width,
        cell_height,
        panel_x: panel.then_some(x + board_width + 2),
    }
}

fn piece_symbol(piece: Piece, color: Color) -> &'static str {
    match (piece, color) {
        (Piece::Pawn, Color::White) => "♙",
        (Piece::Knight, Color::White) => "♘",
        (Piece::Bishop, Color::White) => "♗",
        (Piece::Rook, Color::White) => "♖",
        (Piece::Queen, Color::White) => "♕",
        (Piece::King, Color::White) => "♔",
        (Piece::Pawn, Color::Black) => "♟",
        (Piece::Knight, Color::Black) => "♞",
        (Piece::Bishop, Color::Black) => "♝",
        (Piece::Rook, Color::Black) => "♜",
        (Piece::Queen, Color::Black) => "♛",
        (Piece::King, Color::Black) => "♚",
    }
}
fn opposite(color: Color) -> Color {
    if color == Color::White {
        Color::Black
    } else {
        Color::White
    }
}
fn color_name(color: Color) -> &'static str {
    if color == Color::White {
        "White"
    } else {
        "Black"
    }
}

fn overlay(
    stdout: &mut Stdout,
    width: u16,
    height: u16,
    lines: &[String],
    color: TermColor,
) -> std::io::Result<()> {
    let visible = lines.len().min(height.saturating_sub(4) as usize);
    let start = lines.len().saturating_sub(visible);
    let box_width = lines[start..]
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(20)
        .min(width.saturating_sub(4) as usize)
        + 4;
    let box_height = visible as u16 + 2;
    let x = width.saturating_sub(box_width as u16) / 2;
    let y = height.saturating_sub(box_height) / 2;
    for row in 0..box_height {
        queue!(
            stdout,
            MoveTo(x, y + row),
            SetBackgroundColor(TermColor::Black),
            Print(" ".repeat(box_width))
        )?;
    }
    for (offset, line) in lines[start..].iter().enumerate() {
        queue!(
            stdout,
            MoveTo(x, y + offset as u16 + 1),
            SetForegroundColor(color),
            SetBackgroundColor(TermColor::Black),
            SetAttribute(if offset == 0 {
                Attribute::Bold
            } else {
                Attribute::Reset
            }),
            Print(center(&truncate(line, box_width - 2), box_width)),
            ResetColor,
            SetAttribute(Attribute::Reset)
        )?;
    }
    Ok(())
}
fn center(text: &str, width: usize) -> String {
    let text = truncate(text, width);
    let length = text.chars().count();
    let left = width.saturating_sub(length) / 2;
    format!(
        "{}{}{}",
        " ".repeat(left),
        text,
        " ".repeat(width - left - length)
    )
}
fn truncate(text: &str, width: usize) -> String {
    text.chars().take(width).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn standard_terminal_has_side_panel() {
        assert!(geometry(120, 34).panel_x.is_some());
    }
    #[test]
    fn compact_terminal_keeps_board() {
        let geometry = geometry(32, 12);
        assert_eq!(geometry.cell_width, 4);
        assert_eq!(geometry.cell_height, 1);
    }

    #[test]
    fn visual_up_moves_toward_black_for_white_orientation() {
        let options = Options {
            opponent: Opponent::Human,
            elo: 1600,
            side: crate::cli::Side::White,
            minutes: 10,
            increment: 0,
        };
        let mut app = App::new(options);
        app.game.cursor = 12;
        app.visual_move(0, -1);
        assert_eq!(app.game.cursor, 20);
    }
}
