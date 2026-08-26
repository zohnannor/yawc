#![expect(clippy::missing_errors_doc, reason = "no one cares")]

pub mod game;
pub mod keyboard;
pub mod raw;
pub mod words;

use std::{env, io};

use crate::game::Game;

fn main() {
    let run = || {
        let word = env::args().nth(1);
        let game = Game::new(word.as_deref())?;
        game.main_loop()?;
        io::Result::Ok(())
    };

    if let Err(e) = run() {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
