use std::{
    io::{self, Stdout, stdout},
    ops,
};

use crossterm::{cursor, execute, style, terminal};

#[derive(Debug)]
pub struct Terminal(Stdout);

impl ops::Deref for Terminal {
    type Target = Stdout;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl ops::DerefMut for Terminal {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Terminal {
    pub(crate) fn new() -> io::Result<Self> {
        let mut stdout = stdout();
        terminal::enable_raw_mode()?;
        execute!(
            stdout,
            terminal::EnterAlternateScreen,
            cursor::Hide,
            terminal::Clear(terminal::ClearType::All),
        )?;

        Ok(Self(stdout))
    }
}
impl Drop for Terminal {
    fn drop(&mut self) {
        drop(terminal::disable_raw_mode());
        drop(execute!(
            self.0,
            style::ResetColor,
            cursor::Show,
            terminal::LeaveAlternateScreen
        ));
    }
}
