use std::fmt;

use crossterm::{
    cursor::{self},
    style::{self, Stylize},
};

use crate::game::Match;

pub(crate) struct Keyboard(Vec<(char, Option<Match>)>);

impl Keyboard {
    fn new() -> Self {
        let letters = "qwertyuiopasdfghjklzxcvbnm";
        let mut keyboard = Vec::with_capacity(26);
        for c in letters.chars() {
            keyboard.push((c, None));
        }
        Self(keyboard)
    }
}

impl Keyboard {
    pub(crate) fn mark_letter(&mut self, letter: char, mark: Match) {
        if let Some(m) = self
            .0
            .iter_mut()
            .find_map(|(c, m)| if *c == letter { Some(m) } else { None })
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
        print_row(f, self.0.iter().take(10))?;
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
        print_row(f, self.0.iter().skip(10).take(9))?;
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
        print_row(f, self.0.iter().skip(10).skip(9))?;
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

fn print_row<'a>(
    f: &mut fmt::Formatter,
    row: impl Iterator<Item = &'a (char, Option<Match>)>,
) -> Result<(), fmt::Error> {
    for (c, m) in row {
        write!(
            f,
            " {} │",
            m.map_or_else(
                || c.to_ascii_uppercase().black().on_white(),
                |m| match m {
                    Match::Correct => c.to_ascii_uppercase().black().on_green(),
                    Match::Misplaced => c.to_ascii_uppercase().black().on_yellow(),
                    Match::Incorrect => c.to_ascii_uppercase().white(),
                }
            )
        )?;
    }
    Ok(())
}
