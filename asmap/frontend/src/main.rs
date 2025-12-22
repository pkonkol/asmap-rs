pub mod components;
mod models;

use components::MapContainer;

fn main() {
    // wasm_logger::init(wasm_logger::Config::new(log::Level::Trace));
    // TODO GIS dodatkowa podstrona z detalami
    // albo jakis router reactowy albo po prostu odrebny html, nie wiem
    yew::Renderer::<MapContainer>::new().render();
}
