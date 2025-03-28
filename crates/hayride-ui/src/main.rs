use leptos::prelude::*;

use crate::views::app::App;
pub mod components;
pub mod stores;
pub mod views;

fn main() {
    mount_to_body(App);
}
