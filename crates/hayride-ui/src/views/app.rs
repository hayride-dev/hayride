use leptos::prelude::*;
use leptos_router::components::*;
use leptos_router::path;
use wasm_bindgen::prelude::*;

use super::chat::Chat;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke(cmd: &str, args: JsValue) -> JsValue;
}


#[component]
pub fn App() -> impl IntoView {
        view! {
            <Router>
            <Routes fallback=|| view! { <div>"Page not found"</div> }>
            <Route path=path!("/") view=Chat/>
            </Routes>
            </Router>
        }
    
}
