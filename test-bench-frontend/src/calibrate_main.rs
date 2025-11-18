use test_bench_frontend::CalibrateFrontend;
use yew::prelude::*;

#[function_component(App)]
fn app() -> Html {
    let window = web_sys::window().expect("no global `window` exists");
    let document = window.document().expect("should have a document on window");

    let width: u32 = document
        .get_element_by_id("app")
        .and_then(|el| el.get_attribute("data-width"))
        .and_then(|w| w.parse().ok())
        .unwrap_or(1920);

    let height: u32 = document
        .get_element_by_id("app")
        .and_then(|el| el.get_attribute("data-height"))
        .and_then(|h| h.parse().ok())
        .unwrap_or(1080);

    html! {
        <CalibrateFrontend {width} {height} />
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
