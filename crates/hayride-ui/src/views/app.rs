use leptos::prelude::*;
use leptos_router::components::*;
use leptos_router::path;
use wasm_bindgen::prelude::*;

use super::chat::Chat;
use crate::components::sidebar::Sidebar;
use crate::components::avatar::Avatar;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke(cmd: &str, args: JsValue) -> JsValue;
}

#[component]
pub fn App() -> impl IntoView {
    view! {
        <div class="flex h-screen w-screen">
        <div class="absolute top-0 right-0 m-4 w-12 z-10">
            <Avatar img_src="https://avatars.githubusercontent.com/u/10167943?v=4".to_string()/>
        </div>
        <aside class="fixed top-0 left-0 h-full w-64 bg-base-200">
            <Sidebar/>
        </aside>
        <main id="content"  class="flex flex-col flex-1 ml-64 overflow-auto">
            <Router>
                <Routes fallback=|| view! { <div>"Page not found"</div> }>
                    <Route path=path!("/") view=Chat/>
                </Routes>
            </Router>
        </main>
    </div>
    }
}
