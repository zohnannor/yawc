use std::{
    cmp,
    io::{self, Write},
    thread,
    time::Duration,
};

use crossterm::{
    cursor,
    event::{self, KeyCode, KeyModifiers},
    execute,
    style::{self, Stylize},
    terminal::{self, size},
};
use lazy_regex::regex_replace_all;
use rand::{prelude::SliceRandom, thread_rng};

use crate::{
    keyboard::Keyboard,
    raw::Terminal,
    words::{ACCEPTABLE, WORDS},
};

pub struct Game<'w> {
    secret_word: &'w str,
    guesses: Vec<(String, [Match; 5])>,
    guess: String,
    keyboard: Keyboard,
    term: Terminal,
}

impl Game<'_> {
    #[allow(clippy::missing_panics_doc, clippy::missing_errors_doc)]
    pub fn new() -> io::Result<Self> {
        Ok(Self {
            secret_word: WORDS.choose(&mut thread_rng()).unwrap(),
            guesses: Vec::default(),
            guess: String::default(),
            keyboard: Keyboard::default(),
            term: Terminal::new()?,
        })
    }

    #[allow(clippy::missing_panics_doc, clippy::missing_errors_doc)]
    pub fn main_loop(mut self) -> io::Result<()> {
        'game: loop {
            self.redraw_screen()?;
            self.write_status_bar(&["Type in a word and press Enter! CTRL-C to quit."])?;
            'round: loop {
                loop {
                    match event::read()? {
                        event::Event::Key(k) => match k.code {
                            KeyCode::Char('C' | 'c') if k.modifiers == KeyModifiers::CONTROL => {
                                break 'game
                            }
                            KeyCode::Char(c)
                                if c.is_ascii_alphabetic()
                                    && c.is_ascii_lowercase()
                                    && self.guess.len() < 5 =>
                            {
                                self.guess.push(c);
                            }
                            KeyCode::Backspace => {
                                self.guess.pop();
                            }
                            KeyCode::Enter if self.guess.len() == 5 => {
                                if let Some(GameState::Win | GameState::Loose) = self.guess()? {
                                    break 'round;
                                }
                            }
                            _ => {}
                        },
                        event::Event::Resize(..) => {
                            execute!(self.term, terminal::Clear(terminal::ClearType::All))?;
                        }
                        event::Event::Mouse(_) => {}
                    }

                    self.redraw_screen()?;
                    self.write_status_bar(&["Type in a word and press Enter! CTRL-C to quit."])?;
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
            width / 2 - 11,
            (self.guesses.len() * 2 + 1).try_into().unwrap(),
        );
        execute!(self.term, cursor::MoveTo(pos.0, pos.1))?;
        if is_valid_word(&self.guess) {
            let matches_ = check_word(self.secret_word, &self.guess);

            self.mark_letters(Some(matches_))?;

            self.guesses
                .push((std::mem::take(&mut self.guess), matches_));

            if self.is_win() {
                Ok(Some(GameState::Win))
            } else if self.is_loose() {
                Ok(Some(GameState::Loose))
            } else {
                Ok(None)
            }
        } else {
            self.mark_letters(None)?;
            Ok(None)
        }
    }

    fn mark_letters(&mut self, matches_: Option<[Match; 5]>) -> io::Result<()> {
        if let Some(matches_) = matches_ {
            for (m, c) in matches_.iter().zip(self.guess.chars()) {
                write!(
                    self.term,
                    " {} │",
                    match m {
                        Match::Correct => c.to_ascii_uppercase().black().on_green(),
                        Match::Misplaced => c.to_ascii_uppercase().black().on_yellow(),
                        Match::Incorrect => c.to_ascii_uppercase().white(),
                    }
                )?;
                self.keyboard.mark_letter(c, *m);
            }
            self.term.flush()?;
        } else {
            let (width, _) = size()?;
            let pos = (
                width / 2 - 11,
                (self.guesses.len() * 2 + 1).try_into().unwrap(),
            );
            self.write_status_bar(&["Word is not in the world list!"])?;
            for i in 0..=3 {
                for c in self.guess.chars() {
                    if i % 2 == 0 {
                        write!(self.term, " {} │", c.to_ascii_uppercase().black().on_red())?;
                    } else {
                        write!(self.term, " {} │", c.to_ascii_uppercase().red().on_black())?;
                    }
                }
                self.term.flush()?;
                execute!(self.term, cursor::MoveTo(pos.0, pos.1))?;
                thread::sleep(Duration::from_millis(150));
            }
        }
        Ok(())
    }

    fn start_new_round(&mut self) -> io::Result<()> {
        self.guess.clear();
        self.guesses.clear();
        self.secret_word = WORDS.choose(&mut thread_rng()).unwrap();
        execute!(self.term, terminal::Clear(terminal::ClearType::All))?;
        self.keyboard = Keyboard::default();
        self.draw_grid()?;
        self.write_status_bar(&["Type in a word and press Enter! CTRL-C to quit."])?;
        Ok(())
    }

    fn final_prompt(&mut self) -> io::Result<Option<()>> {
        let (state, word) = if self.is_win() {
            ("won", self.secret_word.green())
        } else {
            ("loose", self.secret_word.red())
        };

        loop {
            self.redraw_screen()?;
            self.write_status_bar(&[
                "You ",
                state,
                "! The word was ",
                &word.to_string(),
                ". Start again? y/n ",
            ])?;
            match event::read()? {
                event::Event::Key(k) => match k.code {
                    KeyCode::Char('y') => return Ok(Some(())),
                    KeyCode::Char('n') => return Ok(None),
                    _ => {}
                },
                event::Event::Resize(..) => {}
                event::Event::Mouse(_) => (),
            }
        }
    }

    fn write_status_bar(&mut self, strings: &[&str]) -> io::Result<()> {
        let (width, height) = size()?;
        match height {
            0..=13 => Ok(()),
            14.. => {
                let height = if height > 14 { height - 2 } else { height };
                let len: u16 = strings
                    .iter()
                    .map(|s| {
                        regex_replace_all!(r"\x1b\[[;\d]*[a-zA-Z]", s, |_| "")
                            .chars()
                            .count()
                    })
                    .sum::<usize>()
                    .try_into()
                    .unwrap();
                execute!(
                    self.term,
                    cursor::SavePosition,
                    cursor::MoveTo(width / 2 - len / 2u16, height),
                    terminal::Clear(terminal::ClearType::CurrentLine),
                )?;
                for string in strings {
                    write!(self.term, "{}", string)?;
                }
                execute!(self.term, cursor::RestorePosition)?;
                Ok(())
            }
        }
    }

    fn display_input(&mut self) -> io::Result<()> {
        let (width, _) = size()?;
        execute!(
            self.term,
            cursor::MoveTo(width / 2 - 11, 1),
            cursor::SavePosition,
        )?;
        for (w, matches_) in &self.guesses {
            for (c, l) in w.chars().zip(matches_) {
                write!(
                    self.term,
                    " {} │",
                    match l {
                        Match::Correct => c.to_ascii_uppercase().black().on_green(),
                        Match::Misplaced => c.to_ascii_uppercase().black().on_yellow(),
                        Match::Incorrect => c.to_ascii_uppercase().white(),
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
        self.guesses.last().unwrap().1 == [Match::Correct; 5] && self.guesses.len() <= 6
    }

    fn is_loose(&self) -> bool {
        self.guesses.last().unwrap().1 != [Match::Correct; 5] && self.guesses.len() >= 6
    }

    fn draw_grid(&mut self) -> io::Result<()> {
        let (width, height) = size()?;

        execute!(self.term, cursor::MoveTo(width / 2 - 12, 0))?;
        execute!(
            self.term,
            cursor::SavePosition,
            style::Print("┌───┬───┬───┬───┬───┐"),
            cursor::RestorePosition,
            cursor::MoveDown(1),
        )?;
        for _ in 0..5 {
            execute!(
                self.term,
                cursor::SavePosition,
                style::Print("│   │   │   │   │   │"),
                cursor::RestorePosition,
                cursor::MoveDown(1),
                cursor::SavePosition,
                style::Print("├───┼───┼───┼───┼───┤"),
                cursor::RestorePosition,
                cursor::MoveDown(1),
            )?;
        }
        execute!(
            self.term,
            cursor::SavePosition,
            style::Print("│   │   │   │   │   │"),
            cursor::RestorePosition,
            cursor::MoveDown(1),
            cursor::SavePosition,
            style::Print("└───┴───┴───┴───┴───┘"),
            cursor::RestorePosition,
        )?;
        if height > 13 + 7 + 1 && width >= 48 {
            let height = if height >= 13 + 1 + 12 {
                height - 12
            } else {
                13
            };
            execute!(self.term, cursor::MoveTo(width / 2 - 20, height))?;
            write!(self.term, "{}", self.keyboard)?;
        }
        Ok(())
    }

    fn redraw_screen(&mut self) -> io::Result<()> {
        let (width, height) = size()?;
        execute!(
            self.term,
            terminal::SetSize(cmp::max(width, 47), cmp::max(height, 13))
        )?;
        self.draw_grid()?;
        self.display_input()?;

        Ok(())
    }
}

enum GameState {
    Win,
    Loose,
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
fn check_word(secret_word: &str, guess: &str) -> [Match; 5] {
    let mut matches = [Match::Incorrect; 5];
    let mut secret_word = secret_word.as_bytes().to_vec();
    // check for correct letters first
    for (i, b) in guess.bytes().enumerate() {
        if secret_word[i] == b {
            secret_word[i] = 0; // remove this letter so that it will not match again
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
            secret_word[j] = 0; // remothe letter from secret word
            matches[i] = Match::Misplaced;
        }
    }

    matches
}
