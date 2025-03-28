use crate::stores::prompt::{Prompt, PromptOptionsStoreFields, PromptStoreFields};
use leptos::prelude::*;
use reactive_stores::Store;
use wasm_bindgen::JsCast;

#[component]
pub fn Config() -> impl IntoView {
    let prompt = expect_context::<Store<Prompt>>();
    // Get the temperature from the global state
    let temperature = prompt.options().temperature();
    // Get system prompt from global state
    let system_prompt = prompt.system();
    // Get the agent from global state
    let agent = prompt.agent();

    view! {
        <div class="flex flex-col">
            <div class="dialog bg-base-100 shadow-md rounded-lg p-4">
                <h1>Agent</h1>
                <div class="mt-4">
                    <input
                        type="text"
                        class="input w-full"
                        placeholder="Example: 'tool_agent'"
                        prop:value=agent.get()
                        on:input=move |e| {
                            if let Some(input) = e.target().and_then(|t| t.dyn_into::<leptos::web_sys::HtmlInputElement>().ok()) {
                                agent.set(input.value());
                            }
                        }
                    />
                </div>
            </div>
            <div class="dialog bg-base-100 shadow-md rounded-lg p-4">
                <h1>System Prompt</h1>
                <div class="mt-4">
                    <textarea
                        class="textarea w-full flex-grow overflow-y-auto resize-none focus:outline-none focus:border-transparent focus:ring-0"
                        placeholder="Example: 'Only answer in rhymes'"
                        prop:value=system_prompt.get()
                        on:input=move |e| {
                            if let Some(input) = e.target().and_then(|t| t.dyn_into::<leptos::web_sys::HtmlTextAreaElement>().ok()) {
                                system_prompt.set(input.value());
                            }
                        }
                    ></textarea>
                </div>
            </div>
            <div class="dialog bg-base-100 shadow-md rounded-lg p-4">
                <div class="mt-4">
                    <input
                        type="range"
                        step="0.01"
                        min="0.0"
                        max="1.0"
                        value=move || temperature.get()
                        on:input=move |e| {
                            if let Some(input) = e.target().and_then(|t| t.dyn_into::<leptos::web_sys::HtmlInputElement>().ok()) {
                                temperature.set(input.value().parse::<f32>().unwrap_or(0.0));
                            }
                        }
                        class="slider"
                    />
                    <p class="text-center mt-2">Temperature: {move || temperature.get()}</p>
                </div>
            </div>
        </div>
    }
}
