use leptos::prelude::*;
use leptos::web_sys::console;
use reactive_stores::Store;

use crate::components::chat::{ChatBubble, ChatMessage, ChatTextArea};
use crate::stores::prompt::Prompt;
use wasm_bindgen_futures::spawn_local;

async fn fetch_prompt(data: String) -> Result<String, Error> {
    let response = reqwasm::http::Request::post("http://localhost:8082/v1/generate")
        .body(data)
        .send()
        .await?;

    // Getting response as a plain text, but could parse json here if needed
    let prompt = response.text().await?;
    Ok(prompt)
}

#[component]
pub fn Chat() -> impl IntoView {
    let (input, set_input) = signal(String::new());
    let (messages, set_messages) = signal(Vec::<ChatMessage>::new());
    let (message_sent, set_message_sent) = signal(false);
    let (sendmsg, set_send_message) = signal(false);

    // When we get a message to send, spawn a task to fetch a prompt
    Effect::new(move |_| {
        if sendmsg.get() {
            let msg = input.get();
            if !msg.is_empty() {
                // console::log_1(&"Sending message".into());

                let mut prompt = expect_context::<Store<Prompt>>().get().clone();
                // Set the prompt message
                prompt.message = msg.clone();

                match serde_json::to_string(&prompt) {
                    Ok(d) => {
                        // Spawn an async task
                        let set_messages = set_messages.clone();
                        let set_input = set_input.clone();
                        let set_message_sent = set_message_sent.clone();

                        spawn_local(async move {
                            // Call the async fetch function
                            match fetch_prompt(d.clone()).await {
                                Ok(response_data) => {
                                    // console::log_1(&format!("Response: {:?}", response_data).into());
    
                                    let message = ChatMessage {
                                        sent: msg.clone(),
                                        response: Some(response_data),
                                    };
    
                                    set_messages.update(|msgs| msgs.push(message));
                                    set_input.set(String::new());
                                    set_message_sent.set(true);
                                }
                                Err(e) => {
                                    console::log_1(&format!("Fetch error: {:?}", e).into());
                                }
                            }
                        });

                    }
                    Err(e) => {
                        console::log_1(&format!("Error serializing prompt data: {:?}", e).into());
                    }
                }
            }
            set_send_message.set(false);
        }
    });

    view! {
          <Show
            when=move || { message_sent.get() }
            fallback= move ||
            view! {
              <div class="flex items-center justify-center min-h-screen">
              <div class="hero">
                  <div class="hero-content text-center">
                      <div class="w-[40vw] h-[35vh] flex flex-grow flex-col ">
                          <h1 class="text-4xl text-base-400 font-bold py-2">"What can I help with?"</h1>
                          <div class="flex flex-col flex-grow p-4">
                            <ChatTextArea input=input set_input=set_input send=set_send_message />
                          </div>
                      </div>
                  </div>
              </div>
          </div>
             }
          >
          <div class="flex flex-col min-h-screen h-full w-full items-center">
              <div class="flex flex-col items-center flex-grow w-full mt-16 max-h-[calc(100vh-16rem)] overflow-y-auto">
                <div class="flex max-w-2xl w-full">
                    <ChatBubble messages=messages.into()/>
                </div>
            </div>
            <div class="fixed w-full max-w-2xl bottom-10">
                <ChatTextArea input=input set_input=set_input send=set_send_message />
            </div>
          </div>
          </Show>
    }
}
