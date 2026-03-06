mod game;

use game::Game;
use macroquad::prelude::*;

fn window_conf() -> Conf {
    Conf {
        window_title: "mcommand".to_owned(),
        fullscreen: !cfg!(target_arch = "wasm32"),
        high_dpi: true,
        sample_count: 4,
        window_resizable: cfg!(target_arch = "wasm32"),
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut game = Game::new(vec2(screen_width(), screen_height())).await;

    loop {
        let dt = get_frame_time().min(1.0 / 30.0);
        if game.update(dt) {
            break;
        }

        game.draw();
        next_frame().await;
    }
}
