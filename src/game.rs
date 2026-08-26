use std::{
    cmp, fmt,
    io::{self, Write as _},
    mem, thread,
    time::Duration,
};

use crossterm::{
    cursor,
    event::{self, KeyCode, KeyModifiers},
    execute,
    style::{self, Stylize as _},
    terminal::{self, size},
};
use rand::seq::IndexedRandom as _;

use crate::{
    keyboard::Keyboard,
    raw::Terminal,
    words::{ACCEPTABLE, WORDS},
};

const WORD_LENGTH: usize = 5;
const MAX_GUESSES: usize = 6;
const INVALID_ANIMATION_DELAY: Duration = Duration::from_millis(150);
const INVALID_ANIMATION_CYCLES: usize = 3;

const GRID_HORIZONTAL_OFFSET: u16 = 12;
const GUESS_HORIZONTAL_OFFSET: u16 = 11;
const STATUS_BAR_MARGIN: u16 = 2;
const KEYBOARD_HORIZONTAL_OFFSET: u16 = 20;
const KEYBOARD_VERTICAL_OFFSET: u16 = GRID_HORIZONTAL_OFFSET;
const MIN_TERMINAL_WIDTH: u16 = 47;
const MIN_TERMINAL_HEIGHT: u16 = 14;
const KEYBOARD_MIN_WIDTH: u16 = MIN_TERMINAL_HEIGHT + 1;
const KEYBOARD_MIN_HEIGHT: u16 = MIN_TERMINAL_HEIGHT + 7 + 1;

const DEFAULT_STATUS_MSG: &str = "Type in a word and press Enter! CTRL-C to quit.";
const NOT_IN_WORD_LIST_MSG: &str = "Word is not in the word list!";
const WIN_PROMPT_PREFIX: &str = "You won! The word was ";
const LOSS_PROMPT_PREFIX: &str = "You lost! The word was ";
const PLAY_AGAIN_PROMPT: &str = ". Start again? y/n (or Enter/Ctrl-C)";
const ERROR_WORDS_EMPTY: &str = "`WORDS` is not empty";
const ERROR_WORD_LENGTH: &str = "Word must be exactly 5 characters long";

const GRID_TOP: &str = "┌───┬───┬───┬───┬───┐";
const GRID_ROW: &str = "│   │   │   │   │   │";
const GRID_MID: &str = "├───┼───┼───┼───┼───┤";
const GRID_BTM: &str = "└───┴───┴───┴───┴───┘";

#[derive(Debug)]
pub struct Game<'word> {
    secret_word: &'word str,
    guesses: Vec<(String, [Match; WORD_LENGTH])>,
    guess: String,
    keyboard: Keyboard,
    term: Terminal,
    stats: Stats,
}

#[derive(Debug, Default)]
struct Stats {
    wins: usize,
    losses: usize,
}

impl<'word> Game<'word> {
    pub fn new(word: Option<&'word str>) -> io::Result<Self> {
        Ok(Self {
            secret_word: match word {
                Some(w) if w.len() == WORD_LENGTH => w,
                Some(_) => {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        ERROR_WORD_LENGTH,
                    ));
                }
                None => WORDS
                    .choose(&mut rand::rng())
                    .ok_or_else::<io::Error, _>(|| unreachable!("{ERROR_WORDS_EMPTY}"))?,
            },
            guesses: Vec::default(),
            guess: String::default(),
            keyboard: Keyboard::default(),
            term: Terminal::new()?,
            stats: Stats::default(),
        })
    }

    #[must_use]
    pub fn stats(&self) -> String {
        format!(
            "[{}/{}/{}] ",
            self.stats.wins.to_string().green(),
            self.stats.losses.to_string().red(),
            self.stats.wins + self.stats.losses
        )
    }

    pub fn main_loop(mut self) -> io::Result<()> {
        'game: loop {
            'round: loop {
                self.redraw_screen()?;
                self.write_status_bar(&[&self.stats(), DEFAULT_STATUS_MSG])?;
                match event::read()? {
                    event::Event::Key(k) => match k.code {
                        KeyCode::Char('C' | 'c') if k.modifiers == KeyModifiers::CONTROL => {
                            break 'game;
                        }
                        KeyCode::Char(c)
                            if c.is_ascii_alphabetic() && self.guess.len() < WORD_LENGTH =>
                        {
                            self.guess.push(c.to_ascii_lowercase());
                        }
                        KeyCode::Backspace => {
                            let _ = self.guess.pop();
                        }
                        KeyCode::Enter if self.guess.len() == WORD_LENGTH => {
                            match self.guess()? {
                                Some(GameState::Win) => {
                                    self.stats.wins += 1;
                                }
                                Some(GameState::Lost) => {
                                    self.stats.losses += 1;
                                }
                                None => continue,
                            }
                            break 'round;
                        }
                        _ => {}
                    },
                    event::Event::Resize(..) => {
                        execute!(self.term, terminal::Clear(terminal::ClearType::All))?;
                    }
                    event::Event::Mouse(_)
                    | event::Event::FocusGained
                    | event::Event::FocusLost
                    | event::Event::Paste(_) => {}
                }
            }

            match self.final_prompt()? {
                Some(()) => self.start_new_round()?,
                None => break 'game,
            }
        }

        Ok(())
    }

    fn guess(&mut self) -> io::Result<Option<GameState>> {
        let (width, _) = size()?;
        let pos = (
            width / 2 - GUESS_HORIZONTAL_OFFSET,
            (self.guesses.len() * 2 + 1).try_into().unwrap_or_default(),
        );
        execute!(self.term, cursor::MoveTo(pos.0, pos.1))?;
        if is_valid_word(&self.guess) {
            let matches = check_word(self.secret_word, &self.guess);

            self.mark_letters(Some(matches))?;

            self.guesses.push((mem::take(&mut self.guess), matches));

            if self.is_win() {
                Ok(Some(GameState::Win))
            } else if self.is_loss() {
                Ok(Some(GameState::Lost))
            } else {
                Ok(None)
            }
        } else {
            self.mark_letters(None)?;
            Ok(None)
        }
    }

    fn mark_letters(&mut self, matches: Option<[Match; WORD_LENGTH]>) -> io::Result<()> {
        if let Some(matches) = matches {
            for (c, m) in self.guess.chars().zip(matches.iter()) {
                write!(
                    self.term,
                    " {} │",
                    match m {
                        Match::Correct => c.black().on_green(),
                        Match::Misplaced => c.black().on_yellow(),
                        Match::Incorrect => c.white(),
                    }
                )?;
                self.keyboard.mark_letter(c, *m);
            }
            self.term.flush()?;
        } else {
            let (width, _) = size()?;
            let pos = (
                width / 2 - GUESS_HORIZONTAL_OFFSET,
                (self.guesses.len() * 2 + 1).try_into().unwrap_or_default(),
            );
            self.write_status_bar(&[&self.stats(), NOT_IN_WORD_LIST_MSG])?;
            for i in 0..=INVALID_ANIMATION_CYCLES {
                for c in self.guess.chars() {
                    let c = c.to_ascii_uppercase();
                    if i % 2 == 0 {
                        write!(self.term, " {} │", c.black().on_red())?;
                    } else {
                        write!(self.term, " {} │", c.red())?;
                    }
                }
                self.term.flush()?;
                execute!(self.term, cursor::MoveTo(pos.0, pos.1))?;
                thread::sleep(INVALID_ANIMATION_DELAY);
            }
        }
        Ok(())
    }

    fn start_new_round(&mut self) -> io::Result<()> {
        self.guess.clear();
        self.guesses.clear();
        self.secret_word = WORDS
            .choose(&mut rand::rng())
            .ok_or_else::<io::Error, _>(|| unreachable!("{ERROR_WORDS_EMPTY}"))?;
        execute!(self.term, terminal::Clear(terminal::ClearType::All))?;
        self.keyboard = Keyboard::default();
        self.draw_grid()?;
        self.write_status_bar(&[&self.stats(), DEFAULT_STATUS_MSG])?;
        Ok(())
    }

    fn final_prompt(&mut self) -> io::Result<Option<()>> {
        let (state_prefix, word) = if self.is_win() {
            (
                WIN_PROMPT_PREFIX,
                self.secret_word.to_ascii_uppercase().green(),
            )
        } else {
            (
                LOSS_PROMPT_PREFIX,
                self.secret_word.to_ascii_uppercase().red(),
            )
        };

        let word = word.underlined().to_string();

        loop {
            self.redraw_screen()?;
            self.write_status_bar(&[&self.stats(), state_prefix, &word, PLAY_AGAIN_PROMPT])?;
            return Ok(match event::read()? {
                event::Event::Key(k) => match k.code {
                    KeyCode::Char('y' | 'Y') | KeyCode::Enter => Some(()),
                    KeyCode::Char('n' | 'N') => None,
                    KeyCode::Char('C' | 'c') if k.modifiers == KeyModifiers::CONTROL => None,
                    _ => continue,
                },
                event::Event::Resize(..) => {
                    execute!(self.term, terminal::Clear(terminal::ClearType::All))?;
                    continue;
                }
                event::Event::Mouse(_)
                | event::Event::FocusGained
                | event::Event::FocusLost
                | event::Event::Paste(_) => continue,
            });
        }
    }

    fn write_status_bar(&mut self, strings: &[&str]) -> io::Result<()> {
        let (width, height) = size()?;
        let height = match height.cmp(&MIN_TERMINAL_HEIGHT) {
            cmp::Ordering::Less => return Ok(()),
            cmp::Ordering::Equal => height,
            cmp::Ordering::Greater => height - STATUS_BAR_MARGIN,
        };
        let len: u16 = strings
            .iter()
            .map(|s| strip_ansi_escapes::strip(s).len())
            .sum::<usize>()
            .try_into()
            .unwrap_or_default();
        execute!(
            self.term,
            cursor::SavePosition,
            cursor::MoveTo(width / 2 - len / 2, height),
            terminal::Clear(terminal::ClearType::CurrentLine),
        )?;
        for string in strings {
            write!(self.term, "{string}")?;
        }
        execute!(self.term, cursor::RestorePosition)?;
        Ok(())
    }

    fn display_input(&mut self) -> io::Result<()> {
        let (width, _) = size()?;
        execute!(
            self.term,
            cursor::MoveTo(width / 2 - GUESS_HORIZONTAL_OFFSET, 1),
            cursor::SavePosition,
        )?;
        for (w, matches) in &self.guesses {
            for (c, m) in w.chars().zip(matches) {
                let c = c.to_ascii_uppercase();
                // unfortunate to_string
                let c = fmt::from_fn(|f| write!(f, " {c} ")).to_string();
                write!(
                    self.term,
                    "{}│",
                    match m {
                        Match::Correct => c.black().on_green(),
                        Match::Misplaced => c.black().on_yellow(),
                        Match::Incorrect => c.white(),
                    }
                )?;
            }
            execute!(
                self.term,
                cursor::RestorePosition,
                cursor::MoveDown(2),
                cursor::SavePosition,
            )?;
        }
        for c in self.guess.chars() {
            write!(self.term, " {} │", c.to_ascii_uppercase().white())?;
            self.term.flush()?;
        }
        Ok(())
    }

    fn is_win(&self) -> bool {
        matches!(
            self.guesses.last(),
            Some((
                _,
                [
                    Match::Correct,
                    Match::Correct,
                    Match::Correct,
                    Match::Correct,
                    Match::Correct,
                ],
            ))
        )
    }

    const fn is_loss(&self) -> bool {
        self.guesses.len() == MAX_GUESSES
    }

    fn draw_grid(&mut self) -> io::Result<()> {
        let (width, height) = size()?;

        execute!(
            self.term,
            cursor::MoveTo(width / 2 - GRID_HORIZONTAL_OFFSET, 0)
        )?;
        execute!(
            self.term,
            cursor::SavePosition,
            style::Print(GRID_TOP),
            cursor::RestorePosition,
            cursor::MoveDown(1),
        )?;
        for _ in 0..WORD_LENGTH {
            execute!(
                self.term,
                cursor::SavePosition,
                style::Print(GRID_ROW),
                cursor::RestorePosition,
                cursor::MoveDown(1),
                cursor::SavePosition,
                style::Print(GRID_MID),
                cursor::RestorePosition,
                cursor::MoveDown(1),
            )?;
        }
        execute!(
            self.term,
            cursor::SavePosition,
            style::Print(GRID_ROW),
            cursor::RestorePosition,
            cursor::MoveDown(1),
            cursor::SavePosition,
            style::Print(GRID_BTM),
            cursor::RestorePosition,
        )?;
        if height > KEYBOARD_MIN_HEIGHT && width >= KEYBOARD_MIN_WIDTH {
            let height = if height >= MIN_TERMINAL_HEIGHT + 1 + KEYBOARD_VERTICAL_OFFSET {
                height - KEYBOARD_VERTICAL_OFFSET
            } else {
                MIN_TERMINAL_HEIGHT
            };
            execute!(
                self.term,
                cursor::MoveTo(width / 2 - KEYBOARD_HORIZONTAL_OFFSET, height)
            )?;
            write!(self.term, "{}", self.keyboard)?;
        }
        Ok(())
    }

    fn redraw_screen(&mut self) -> io::Result<()> {
        let (width, height) = size()?;
        execute!(
            self.term,
            terminal::SetSize(
                cmp::max(width, MIN_TERMINAL_WIDTH),
                cmp::max(height, MIN_TERMINAL_HEIGHT)
            )
        )?;
        self.draw_grid()?;
        self.display_input()?;

        Ok(())
    }
}

enum GameState {
    Win,
    Lost,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Match {
    Correct,
    Misplaced,
    Incorrect,
}

#[must_use]
fn is_valid_word(word: &str) -> bool {
    WORDS.contains(&word) || ACCEPTABLE.contains(&word)
}

#[must_use]
fn check_word(secret_word: &str, guess: &str) -> [Match; WORD_LENGTH] {
    let mut matches = [Match::Incorrect; WORD_LENGTH];
    let mut secret_word = secret_word.as_bytes().to_vec();
    // check for correct letters first
    for (i, b) in guess.bytes().enumerate() {
        if secret_word[i] == b {
            // remove this letter so that it will not match again
            secret_word[i] = 0;
            matches[i] = Match::Correct;
        }
    }
    // then check for misplaced letters:
    for (i, c) in guess.bytes().enumerate() {
        if matches[i] != Match::Incorrect {
            continue; // skip all correct letters
        }
        // find first occurrence of current letter in the secret word
        if let Some(j) = secret_word.iter().position(|&b| c == b) {
            secret_word[j] = 0; // remove the letter from secret word
            matches[i] = Match::Misplaced;
        }
    }

    matches
}
