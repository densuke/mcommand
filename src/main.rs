mod game;

use game::Game;
use macroquad::prelude::*;

fn window_conf() -> Conf {
    Conf {
        window_title: "mcommand".to_owned(),
        fullscreen: true,
        high_dpi: true,
        sample_count: 4,
        window_resizable: false,
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
