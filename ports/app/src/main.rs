mod app;
mod components;
mod pages;
mod router;
mod store;
mod svc;
mod utils;

fn main() {
    yew::Renderer::<app::App>::new().render();
}
