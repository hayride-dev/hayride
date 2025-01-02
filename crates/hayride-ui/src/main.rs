
use leptos::prelude::*;

use crate::views::app::App;
pub mod views;
pub mod components;

fn main() {
    mount_to_body(App);
}
