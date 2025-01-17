use leptos::prelude::*;
use leptos_router::components::*;
use leptos_router::path;
use wasm_bindgen::prelude::*;
use reactive_stores::Store;
use serde::{Deserialize, Serialize};

use super::chat::Chat;
use crate::components::sidebar::Sidebar;
use crate::components::config::Config;
use crate::components::avatar::Avatar;

#[derive(Clone, Serialize, Deserialize, Default, Store)]
pub struct Prompt {
    pub message: String,
    system: String,
    agent: String,
    options: PromptOptions,
}

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
    // Set a global system prompt and options state
    let mut prompt = Prompt::default();

    // Override some default options that are not yet available in the UI
    prompt.agent ="tool_agent".to_string();
    prompt.options.num_batch = 4096;
    prompt.options.num_context = 4096;
    prompt.options.top_k = 20;
    prompt.options.top_p = 0.9;
    provide_context(Store::new(prompt));

    view! {
        <div class="flex h-screen w-screen">
        <div class="absolute top-0 right-0 m-4 w-12 z-10">
            <Avatar img_src="https://avatars.githubusercontent.com/u/10167943?v=4".to_string()/>
        </div>
        <aside class="fixed top-0 left-0 h-full w-64 bg-base-200 rounded-r-xl">
            <Sidebar/>
        </aside>
        <aside class="fixed top-0 right-0 h-full w-64 bg-base-200 rounded-l-xl">
            <Config/>
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
