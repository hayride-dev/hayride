use leptos::prelude::*;
/// Renders the home page of your application.
#[component]
pub fn HomePage() -> impl IntoView {
    // Creates a reactive value to update the button
    let count = RwSignal::new(0); // Use `RwSignal::new()` to initialize the signal
    let on_click = move |_| count.update(|c| *c += 1); // Use `update` for modifying the signal

    view! {
        <h1>"Welcome to Leptos!"</h1>
        <button on:click=on_click>
            "Click Me: " {move || count.get()} // Wrap `count.get()` in a closure
        </button>
    }
}