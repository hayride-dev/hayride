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
    max_predict: i32,
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
    prompt.options.num_batch = 20000;
    prompt.options.num_context = 20000;
    prompt.options.max_predict = 2000; // Limit the number of tokens the llm can generate
    prompt.options.top_k = 20;
    prompt.options.top_p = 0.9;
    provide_context(Store::new(prompt));

    view! {
        <div class="flex h-screen w-screen bg-neutral">
        <div class="drawer drawer-end">
            <input id="chat-settings" type="checkbox" class="drawer-toggle" />
            <div class="drawer-content flex flex-col">
                <aside class="fixed top-0 left-0 h-full w-64 rounded-r-xl border-r-2 bg-base-100">
                    <Sidebar/>
                </aside>

                <div class="fixed top-2 right-2 space-x-4 flex justify-end">
                    <label for="chat-settings" class="ma-2 drawer-button btn btn-ghost">
                        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="size-6">
                            <path stroke-linecap="round" stroke-linejoin="round" d="M11.42 15.17 17.25 21A2.652 2.652 0 0 0 21 17.25l-5.877-5.877M11.42 15.17l2.496-3.03c.317-.384.74-.626 1.208-.766M11.42 15.17l-4.655 5.653a2.548 2.548 0 1 1-3.586-3.586l6.837-5.63m5.108-.233c.55-.164 1.163-.188 1.743-.14a4.5 4.5 0 0 0 4.486-6.336l-3.276 3.277a3.004 3.004 0 0 1-2.25-2.25l3.276-3.276a4.5 4.5 0 0 0-6.336 4.486c.091 1.076-.071 2.264-.904 2.95l-.102.085m-1.745 1.437L5.909 7.5H4.5L2.25 3.75l1.5-1.5L7.5 4.5v1.409l4.26 4.26m-1.745 1.437 1.745-1.437m6.615 8.206L15.75 15.75M4.867 19.125h.008v.008h-.008v-.008Z" />
                        </svg>
                    </label>
                    <Avatar img_src="https://avatars.githubusercontent.com/u/10167943?v=4".to_string()/>
                </div>

                <main id="content" class="bg-neutral flex flex-col flex-1 ml-64 max-w-full">
                    <Router>
                        <Routes fallback=|| view! { <div>"Page not found"</div> }>
                            <Route path=path!("/") view=Chat/>
                        </Routes>
                    </Router>
                </main>
            </div>
            <div class="drawer-side">
                <label for="chat-settings" aria-label="close sidebar" class="drawer-overlay"></label>
                <div class="h-full w-96 rounded-l-xl border-r-2 bg-base-200">
                    <Config/>
                </div>
            </div>
        </div>
    </div>
    }
}
