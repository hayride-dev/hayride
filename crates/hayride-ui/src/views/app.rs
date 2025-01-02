use leptos::prelude::*;
use leptos_router::components::*;
use leptos_router::path;
use wasm_bindgen::prelude::*;
use reactive_stores::Store;
use serde::{Deserialize, Serialize};

use super::chat::Chat;
use crate::components::sidebar::Sidebar;
use crate::components::avatar::Avatar;

// Global State for prompt options
#[derive(Clone, Serialize, Deserialize, Default, Store)]
pub struct PromptOptions {
    temperature: f32,
    num_context: i32,
    num_batch: i32,
    top_k: i32,
    top_p: f32,
    seed: u32,
}


#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = ["window", "__TAURI__", "core"])]
    async fn invoke(cmd: &str, args: JsValue) -> JsValue;
}

#[component]
pub fn App() -> impl IntoView {
    // Override some default options that are not yet available in the UI
    let mut options = PromptOptions::default();
    options.num_batch = 2048;
    options.num_context = 2048;
    options.top_k = 20;
    options.top_p = 0.9;
    provide_context(Store::new(options));

    view! {
        <div class="flex h-screen w-screen">
        <div class="absolute top-0 right-0 m-4 w-12 z-10">
            <Avatar img_src="https://avatars.githubusercontent.com/u/10167943?v=4".to_string()/>
        </div>
        <aside class="fixed top-0 left-0 h-full w-64 bg-base-200 rounded-r-xl">
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
