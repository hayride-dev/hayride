use leptos::prelude::*;
use leptos_router::components::*;
use leptos_router::path;
use wasm_bindgen::prelude::*;

use super::chat::Chat;
use crate::components::sidebar::Sidebar;


#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke(cmd: &str, args: JsValue) -> JsValue;
}

#[component]
pub fn App() -> impl IntoView {
    view! {
        <div class="flex h-screen w-screen">
        <aside class="fixed top-0 left-0 h-full w-64 bg-base-200">
            <Sidebar/>
        </aside>
        <main id="content" class="flex-1 ml-64 overflow-auto">
            <Router>
                <Routes fallback=|| view! { <div>"Page not found"</div> }>
                    <Route path=path!("/") view=Chat/>
                </Routes>
            </Router>
        </main>
    </div>
    }
}
