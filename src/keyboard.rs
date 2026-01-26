use std::{fmt, iter::repeat};

use crossterm::{
    cursor::{self},
    style::{self, Stylize as _},
};

use crate::game::Match;

#[derive(Debug)]
pub(crate) struct Keyboard(Vec<(char, Option<Match>)>);

impl Keyboard {
    fn new() -> Self {
        Self(
            "qwertyuiopasdfghjklzxcvbnm"
                .chars()
                .zip(repeat(None))
                .collect(),
        )
    }

    pub(crate) fn mark_letter(&mut self, letter: char, mark: Match) {
        if let Some(m) = self
            .0
            .iter_mut()
            .find_map(|(c, m)| (*c == letter).then_some(m))
        {
            *m = Some(mark);
        }
    }
}

impl Default for Keyboard {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for Keyboard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut it = self.0.iter();
        let it = it.by_ref();
        write!(
            f,
            "{}{}{}{}{}{}",
            cursor::SavePosition,
            style::Print("┌───┬───┬───┬───┬───┬───┬───┬───┬───┬───┐"),
            cursor::RestorePosition,
            cursor::MoveDown(1),
            cursor::SavePosition,
            style::Print("│"),
        )?;
        print_row(f, it.take(10))?;
        write!(
            f,
            "{}{}{}{}{}{}{}{}",
            cursor::RestorePosition,
            cursor::MoveDown(1),
            cursor::SavePosition,
            style::Print("└─┬─┴─┬─┴─┬─┴─┬─┴─┬─┴─┬─┴─┬─┴─┬─┴─┬─┴─┬─┘"),
            cursor::RestorePosition,
            cursor::MoveDown(1),
            cursor::SavePosition,
            style::Print("  │"),
        )?;
        print_row(f, it.take(9))?;
        write!(
            f,
            "{}{}{}{}{}{}{}{}",
            cursor::RestorePosition,
            cursor::MoveDown(1),
            cursor::SavePosition,
            style::Print("  └─┬─┴─┬─┴─┬─┴─┬─┴─┬─┴─┬─┴─┬─┴─┬─┴───┘"),
            cursor::RestorePosition,
            cursor::MoveDown(1),
            cursor::SavePosition,
            style::Print("    │"),
        )?;
        print_row(f, it)?;
        write!(
            f,
            "{}{}{}{}",
            cursor::RestorePosition,
            cursor::MoveDown(1),
            cursor::SavePosition,
            style::Print("    └───┴───┴───┴───┴───┴───┴───┘")
        )?;
        Ok(())
    }
}

fn print_row<'item, I>(f: &mut fmt::Formatter<'_>, row: I) -> fmt::Result
where
    I: Iterator<Item = &'item (char, Option<Match>)>,
{
    for (c, m) in row {
        let c = c.to_ascii_uppercase();
        // unfortunate to_string
        let c = fmt::from_fn(|f| write!(f, " {c} ")).to_string();
        write!(
            f,
            "{}│",
            match m {
                Some(m) => match m {
                    Match::Correct => c.black().on_green(),
                    Match::Misplaced => c.black().on_yellow(),
                    Match::Incorrect => c.grey().crossed_out().dim(),
                },
                None => c.white().bold(),
            }
        )?;
    }
    Ok(())
}
