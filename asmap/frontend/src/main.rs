mod components;
mod models;
mod worker;

use components::MapContainer;

fn main() {
    // wasm_logger::init(wasm_logger::Config::new(log::Level::Trace));
    yew::Renderer::<MapContainer>::new().render();
}
