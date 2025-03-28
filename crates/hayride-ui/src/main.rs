
use leptos::prelude::*;

use crate::views::app::App;
pub mod views;
pub mod components;
pub mod stores;

fn main() {
    mount_to_body(App);
}
