mod app;
mod graph;
mod osm;
mod router;
mod types;
mod ui;

use app::NavigationApp;
use macroquad::prelude::*;
use types::{SCREEN_HEIGHT, SCREEN_WIDTH};

fn window_conf() -> Conf {
    Conf {
        window_title: "OSM Navigation".to_string(),
        window_width: SCREEN_WIDTH as i32,
        window_height: SCREEN_HEIGHT as i32,
        window_resizable: false,
        sample_count: 4,
        ..Default::default()
    }
}

#[macroquad::main(window_conf)]
async fn main() {
    let mut app = NavigationApp::new("map.osm.pbf");

    loop {
        clear_background(Color::from_rgba(12, 16, 24, 255));

        if is_key_pressed(KeyCode::Escape) {
            break;
        }

        app.handle_input();
        app.update();
        app.draw();

        next_frame().await;
    }
}