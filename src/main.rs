use std::io;

use yawc::game::Game;

fn main() {
    let run = || {
        let game = Game::new()?;
        game.main_loop()?;
        io::Result::Ok(())
    };

    if let Err(e) = run() {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
