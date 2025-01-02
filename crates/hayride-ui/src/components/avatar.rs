use leptos::prelude::*;

#[component]
pub fn Avatar(img_src: String) -> impl IntoView {
    view! {
        <div class="dropdown dropdown-end">
        <div tabindex="0" role="button" class="btn btn-circle">
            <div class="avatar">
                <div class="ring-primary ring-offset-base-100 rounded-full ring ring-offset-0">
                    <img src={img_src} alt="Avatar" class="w-full h-full object-cover rounded-full" />
                </div>
            </div>
        </div>
        <ul tabindex="0" class="dropdown-content menu bg-base-100 rounded-box z-[1] w-52 p-2 shadow">
            <li><a>Item 1</a></li>
            <li><a>Item 2</a></li>
        </ul>
    </div>
    }
}