pub mod bindings;
pub mod wac;
mod wac_impl;

pub use wac::WacCtx;
pub use wac::{WacImpl, WacView};

use hayride_host_traits::wac::WacTrait;

pub fn add_to_linker_sync<T>(l: &mut wasmtime::component::Linker<T>) -> anyhow::Result<()>
where
    T: WacView,
{
    let closure = type_annotate_silo::<T, _>(|t| WacImpl(t));
    crate::wac::bindings::wac::add_to_linker_get_host(l, closure)?;
    crate::wac::bindings::types::add_to_linker_get_host(l, closure)?;

    Ok(())
}

// NB: workaround some rustc inference - a future refactoring may make this
// obsolete.
fn type_annotate_silo<T, F>(val: F) -> F
where
    F: Fn(&mut T) -> WacImpl<&mut T>,
{
    val
}

pub struct WacBackend(Box<dyn WacTrait>);
impl std::ops::Deref for WacBackend {
    type Target = dyn WacTrait;
    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}
impl std::ops::DerefMut for WacBackend {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.as_mut()
    }
}
impl<T: WacTrait + 'static> From<T> for WacBackend {
    fn from(value: T) -> Self {
        Self(Box::new(value))
    }
}
