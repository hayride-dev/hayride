use leptos::prelude::*;
use leptos::web_sys::console;

#[derive(Clone)]
pub struct ChatMessage {
    pub sent: String,
    pub response: Option<String>,
}

#[component]
pub fn ChatTextArea(input: ReadSignal<String>, set_input: WriteSignal<String>, send: WriteSignal<bool>) -> impl IntoView {

    let on_click = move |_ev: leptos::ev::MouseEvent| {
        console::log_1(&"Received message".into());
        send.set(true);
    };

    let on_keydown = move |ev: leptos::ev::KeyboardEvent| {
        if ev.key() == "Enter" && !ev.shift_key() { // Check if Enter is pressed and Shift is not held
            ev.prevent_default();
            on_click(leptos::ev::MouseEvent::new("click").unwrap());
        }
    };

    view! {
        <div class="border-2 border-neutral-600 rounded-lg h-full w-full flex flex-col flex-grow">
            <textarea
                class="textarea w-full flex-grow overflow-y-auto resize-none focus:outline-none focus:border-transparent focus:ring-0" 
                placeholder="Ask anything..."
                prop:value=input
                on:input=move |ev| set_input.set(event_target_value(&ev))
                on:keydown=on_keydown
            >
                {input.get()}
            </textarea>
            <div class="w-full flex justify-between p-2">
                <button class="btn btn-ghost">
                    <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="2" stroke="currentColor" class="size-5">
                        <path stroke-linecap="round" stroke-linejoin="round" d="m18.375 12.739-7.693 7.693a4.5 4.5 0 0 1-6.364-6.364l10.94-10.94A3 3 0 1 1 19.5 7.372L8.552 18.32m.009-.01-.01.01m5.699-9.941-7.81 7.81a1.5 1.5 0 0 0 2.112 2.13" />
                    </svg>
                </button>
                <button class="btn btn-ghost" on:click=on_click>
                    <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="size-6">
                        <path stroke-linecap="round" stroke-linejoin="round" d="m15 11.25-3-3m0 0-3 3m3-3v7.5M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0Z" />
                    </svg>
                </button>
            </div>
        </div>
    }
}

#[component]
pub fn ChatBubble(messages:Signal<Vec<ChatMessage>>) -> impl IntoView {
    view! {
        <div class="flex flex-col h-full w-full overflow-y-auto">
            <p class="mt-3 text-gray-600 dark:text-neutral-400">
                {move || messages.get().iter().map(|msg| view! {
                    <div class="chat chat-end">
                        <div class="chat-bubble chat-bubble-secondary">
                            {msg.sent.clone()}
                        </div>
                    </div>
                    <div class="chat chat-start">
                        <div class="chat-bubble chat-bubble-primary-content">
                            {msg.response.clone()} 
                        </div>
                    </div>
                }).collect::<Vec<_>>()}
            </p>
        </div>
    }
}