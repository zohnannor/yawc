use crossterm::{
    cursor, execute,
    style::{self, Color},
    terminal,
};

use std::io::{self, stdout, Stdout};

pub struct Terminal(Stdout);

impl std::ops::Deref for Terminal {
    type Target = Stdout;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for Terminal {
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
            style::SetBackgroundColor(Color::Black)
        )?;

        Ok(Self(stdout))
    }
}
impl Drop for Terminal {
    fn drop(&mut self) {
        terminal::disable_raw_mode().ok();
        execute!(
            self.0,
            style::ResetColor,
            cursor::Show,
            terminal::LeaveAlternateScreen
        )
        .ok();
    }
}
