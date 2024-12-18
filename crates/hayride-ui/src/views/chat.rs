use leptos::prelude::*;
use leptos::web_sys::console;
use leptos_use::{
use_websocket, UseWebSocketReturn,
};
use codee::string::FromToStringCodec;

use crate::components::chat::{ChatTextArea, ChatBubble, ChatMessage};

#[component]
pub fn Chat() -> impl IntoView {
    let UseWebSocketReturn {
        ready_state,
        message,
        send,
        ..
    } = use_websocket::<String, String, FromToStringCodec>("wss://echo.websocket.events/");

    let (input, set_input) = signal(String::new());
    let (messages, set_messages) = signal(Vec::<ChatMessage>::new());
    let (message_sent, set_message_sent) = signal(false);
    let (sendmsg, set_send_message) = signal(false);
    // Update the messages signal when a new message is received
    Effect::new(move |_| {
        if let Some(msg) = message.get() {
            console::log_1(&"Received message".into());
            set_messages.update(|msgs| {
              if let Some(last_message) = msgs.last_mut() {
                match &last_message.response {
                  Some(m) => {
                    // Append to previous response
                    last_message.response = Some(format!("{}{}", m, msg));
                  },
                  None => {
                    // Set response if not already set
                    last_message.response = Some(msg);
                  }
                }
              } else {
                console::log_1(&"No Sent Message Yet".into());
              }
          });
        }
    });

    Effect::new(move |_| {
        console::log_1(&format!("Current message: {:?}", message.get()).into());
    });

    Effect::new(move |_| {
        console::log_1(&format!("WebSocket Ready State: {:?}", ready_state.get()).into());
    });

    Effect::new(move |_| {
      if sendmsg.get() {
      let msg = input.get();
        if !msg.is_empty() {
            console::log_1(&"Sending message".into());

            send(&msg.clone());

            let message = ChatMessage {
              sent: msg,
              response: None,
            };

            set_messages.update(|msgs| msgs.push(message));
            set_input.set(String::new());
            // set_history.update(|history: &mut Vec<_>| history.push(format!("[send]: {:?}", m)));
            set_message_sent.set(true); // Update the message_sent state
        }
          set_send_message.set(false);
        }
    });

    view! {
          <Show
            when=move || { message_sent.get() }
            fallback= move || 
            view! { 
              <div class="hero">
                <div class="hero-content text-center">
                  <div class="max-w-xl">
                    <h1 class="text-4xl font-bold">"What can I help with?"</h1>
                    <div class="bg-base-300 py-6 h-80 overflow-y-auto flex flex-col flex-grow">
                    <ChatTextArea input=input set_input=set_input send=set_send_message />
                </div>
                  </div>
                </div>
              </div>
             }
          >
          <div class="flex flex-col h-full w-full items-center justify-between">
          <div class="flex-grow w-full max-w-2xl pt-4">
              <ChatBubble messages=messages.into()/>
          </div>
          <div class="w-full max-w-2xl sticky bottom-0 pb-4 bg-blue-700">
              <ChatTextArea input=input set_input=set_input send=set_send_message />
          </div>
      </div>
          </Show>
    }
}
