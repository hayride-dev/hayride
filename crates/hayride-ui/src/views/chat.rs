use leptos::prelude::*;
use leptos::web_sys::console;
use leptos_use::{
  core::ConnectionReadyState, use_websocket, UseWebSocketReturn,
};
use codee::string::FromToStringCodec;

#[component]
pub fn Chat() -> impl IntoView {
    // ----------------------------
    // use_websocket
    // ----------------------------

    let UseWebSocketReturn {
        ready_state,
        message,
        send,
        open,
        close,
        ..
    } = use_websocket::<String, String, FromToStringCodec>("wss://echo.websocket.events/");

    let (input, set_input) = signal(String::new());
    let (messages, set_messages) = signal(Vec::new());

    // Update the messages signal when a new message is received
    Effect::new(move |_| {
        if let Some(msg) = message.get() {
            console::log_1(&"Received message".into());
            set_messages.update(|msgs| msgs.push(format!("{}", msg)));
        }
    });

    Effect::new(move |_| {
        console::log_1(&format!("Current message: {:?}", message.get()).into());
    });

    Effect::new(move |_| {
        console::log_1(&format!("WebSocket Ready State: {:?}", ready_state.get()).into());
    });

    let send_message = move |_| {
      let msg = input.get();
        if !msg.is_empty() {
            console::log_1(&"Sending message".into());

            send(&msg.clone());
            set_input.set(String::new());
            // set_history.update(|history: &mut Vec<_>| history.push(format!("[send]: {:?}", m)));
        }
    };

    let status = move || ready_state.get().to_string();

    let connected = move || ready_state.get() == ConnectionReadyState::Open;

    view! {
            <div class="relative h-screen">
                <div class="max-w-4xl px-4 py-10 sm:px-6 lg:px-8 lg:py-14 mx-auto">
                    <div class="text-center">
                        <h1 class="text-3xl font-bold text-gray-800 sm:text-4xl dark:text-white">
                            "Welcome to Preline AI"
                        </h1>
                        <p class="mt-3 text-gray-600 dark:text-neutral-400">
                            "Your AI-powered copilot for the web"
                        </p>
                    </div>
                </div>

                <h2>"Messages"</h2>
                <p class="mt-3 text-gray-600 dark:text-neutral-400">
                  {move || messages.get().iter().map(|msg| view! { <li>{msg.clone()}</li> }).collect::<Vec<_>>()}
                </p>

                // Input -->
                <div class="relative">
                  <input 
                    value=input
                    on:input=move |ev| set_input.set(event_target_value(&ev))
                    class="p-4 pb-12 block w-full border-gray-200 rounded-lg text-sm focus:border-blue-500 focus:ring-blue-500 disabled:opacity-50 disabled:pointer-events-none dark:bg-neutral-900 dark:border-neutral-700 dark:text-neutral-400 dark:placeholder-neutral-500 dark:focus:ring-neutral-600"
                    placeholder="Ask me anything..."
                  >
                  </input>

                  // Toolbar -->
                  <div class="absolute bottom-px inset-x-px p-2 rounded-b-lg bg-white dark:bg-neutral-900">
                    <div class="flex justify-between items-center">
                      // Button Group -->
                      <div class="flex items-center">
                        // Mic Button -->
                        <button type="button" class="inline-flex shrink-0 justify-center items-center size-8 rounded-lg text-gray-500 hover:bg-gray-100 focus:z-10 focus:outline-none focus:bg-gray-100 dark:text-neutral-500 dark:hover:bg-neutral-700 dark:focus:bg-neutral-700">
                          <svg class="shrink-0 size-4" xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                            <rect width="18" height="18" x="3" y="3" rx="2" />
                            <line x1="9" x2="15" y1="15" y2="9" />
                          </svg>
                        </button>
                        // End Mic Button -->

                        // Attach Button -->
                        <button type="button" class="inline-flex shrink-0 justify-center items-center size-8 rounded-lg text-gray-500 hover:bg-gray-100 focus:z-10 focus:outline-none focus:bg-gray-100 dark:text-neutral-500 dark:hover:bg-neutral-700 dark:focus:bg-neutral-700">
                          <svg class="shrink-0 size-4" xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                            <path d="m21.44 11.05-9.19 9.19a6 6 0 0 1-8.49-8.49l8.57-8.57A4 4 0 1 1 18 8.84l-8.59 8.57a2 2 0 0 1-2.83-2.83l8.49-8.48" />
                          </svg>
                        </button>
                        // End Attach Button -->
                      </div>
                      // End Button Group -->

                      // Button Group -->
                      <div class="flex items-center gap-x-1">
                        // Mic Button -->
                        <button type="button" class="inline-flex shrink-0 justify-center items-center size-8 rounded-lg text-gray-500 hover:bg-gray-100 focus:z-10 focus:outline-none focus:bg-gray-100 dark:text-neutral-500 dark:hover:bg-neutral-700 dark:focus:bg-neutral-700">
                          <svg class="shrink-0 size-4" xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                            <path d="M12 2a3 3 0 0 0-3 3v7a3 3 0 0 0 6 0V5a3 3 0 0 0-3-3Z" />
                            <path d="M19 10v2a7 7 0 0 1-14 0v-2" />
                            <line x1="12" x2="12" y1="19" y2="22" />
                          </svg>
                        </button>
                        // End Mic Button -->

                        // Send Button -->
                        <button type="button" on:click=send_message class="inline-flex shrink-0 justify-center items-center size-8 rounded-lg text-white bg-blue-600 hover:bg-blue-500 focus:z-10 focus:outline-none focus:bg-blue-500">
                          <svg class="shrink-0 size-3.5" xmlns="http://www.w3.org/2000/svg" width="16" height="16" fill="currentColor" viewBox="0 0 16 16">
                            <path d="M15.964.686a.5.5 0 0 0-.65-.65L.767 5.855H.766l-.452.18a.5.5 0 0 0-.082.887l.41.26.001.002 4.995 3.178 3.178 4.995.002.002.26.41a.5.5 0 0 0 .886-.083l6-15Zm-1.833 1.89L6.637 10.07l-.215-.338a.5.5 0 0 0-.154-.154l-.338-.215 7.494-7.494 1.178-.471-.47 1.178Z" />
                          </svg>
                        </button>
                        // End Send Button -->
                      </div>
                      // End Button Group -->
                    </div>
                  </div>
                  // End Toolbar -->
                </div>
                // End Input -->
            </div>
          <details class="dropdown">
          <summary class="btn m-1">open or close</summary>
          <ul class="menu dropdown-content bg-base-100 rounded-box z-[1] w-52 p-2 shadow">
            <li><a>Item 1</a></li>
            <li><a>Item 2</a></li>
          </ul>
        </details>
    }
}
