use in_falsus_editor::app::setup::window_conf;

#[macroquad::main(window_conf)]
async fn main() {
    in_falsus_editor::run_app().await;
}
