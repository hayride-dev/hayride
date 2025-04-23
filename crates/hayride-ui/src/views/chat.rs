use leptos::prelude::*;
use leptos::web_sys::console;
use reactive_stores::Store;

use crate::components::chat::{ChatBubble, ChatMessage, ChatTextArea};
use crate::stores::prompt::Prompt;
use crate::stores::bindings::{Request, Response, Data, Message, Role, Content, TextContent};
use wasm_bindgen_futures::spawn_local;

async fn fetch_generate(data: String) -> Result<Response, Error> {
    let response = reqwasm::http::Request::post("http://localhost:8082/v1/generate")
        .body(data)
        .send()
        .await?;

    // Getting response as a plain text, but could parse json here if needed
    let prompt = response.json::<Response>()
        .await?;
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

                let prompt = expect_context::<Store<Prompt>>().get().clone();

                // Set metadata based on prompt options
                // metadata is a list of tuple string:string values
                let metadata = vec![
                    ("temperature".to_string(), prompt.options.temperature.to_string()),
                    ("num_context".to_string(), prompt.options.num_context.to_string()),
                    ("num_batch".to_string(), prompt.options.num_batch.to_string()),
                    ("max_predict".to_string(), prompt.options.max_predict.to_string()),
                    ("top_k".to_string(), prompt.options.top_k.to_string()),
                    ("top_p".to_string(), prompt.options.top_p.to_string()),
                    ("seed".to_string(), prompt.options.seed.to_string()),
                    ("agent".to_string(), prompt.agent.clone()),
                ];

                // Create a new message with the user role
                let message = Message {
                    role: Role::User,
                    content: vec![Content::Text(TextContent { text: msg.clone(), content_type: "text".to_string() })],
                };

                let request = Request {
                    // role: Role::User,
                    // content: vec![msg.clone()],
                    data: Data::Messages(vec![message]),
                    metadata: metadata,
                };

                match serde_json::to_string(&request) {
                    Ok(d) => {
                        // Spawn an async task
                        let set_messages = set_messages.clone();
                        let set_input = set_input.clone();
                        let set_message_sent = set_message_sent.clone();

                        spawn_local(async move {
                            // Call the async fetch function
                            match fetch_generate(d.clone()).await {
                                Ok(response_data) => {
                                    // console::log_1(&format!("Response: {:?}", response_data).into());
                                    if response_data.error.len() > 0 {
                                        console::log_1(&format!("Error in response: {:?}", response_data.error).into());
                                        return;
                                    }

                                    let data = response_data.data;
                                    match data {
                                        Data::Messages(messages) => {
                                            // console::log_1(&format!("Received messages: {:?}", messages).into());
                                            // Convert messages to ChatMessage
                                            let chat_messages: Vec<ChatMessage> = messages.into_iter().map(|m| {
                                                let response = m.content.into_iter().find_map(|c| {
                                                    if let Content::Text(t) = c {
                                                        Some(t.text)
                                                    } else {
                                                        None
                                                    }
                                                }).unwrap_or_default();

                                                ChatMessage {
                                                    sent: msg.clone(),
                                                    response: Some(response),
                                                }
                                            }).collect();
    
                                            set_messages.update(|msgs| msgs.extend(chat_messages));
                                            set_input.set(String::new());
                                            set_message_sent.set(true);
                                        },
                                        _ => {
                                            console::log_1(&"Unexpected data format".into());
                                        }
                                    }
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
