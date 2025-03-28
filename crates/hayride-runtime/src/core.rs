pub mod bindings;
pub mod core;
mod core_impl;

pub use core::{CoreCtx, CoreImpl, CoreView};

use hayride_host_traits::core::ConfigTrait;

pub fn add_to_linker_sync<T>(l: &mut wasmtime::component::Linker<T>) -> anyhow::Result<()>
where
    T: CoreView,
{
    let closure = type_annotate_core::<T, _>(|t| CoreImpl(t));
    crate::core::bindings::types::add_to_linker_get_host(l, closure)?;
    crate::core::bindings::config::add_to_linker_get_host(l, closure)?;

    Ok(())
}

// NB: workaround some rustc inference - a future refactoring may make this
// obsolete.
fn type_annotate_core<T, F>(val: F) -> F
where
    F: Fn(&mut T) -> CoreImpl<&mut T>,
{
    val
}

pub struct CoreBackend(Box<dyn ConfigTrait>);
impl std::ops::Deref for CoreBackend {
    type Target = dyn ConfigTrait;
    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}
impl std::ops::DerefMut for CoreBackend {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.as_mut()
    }
}
impl<T: ConfigTrait + 'static> From<T> for CoreBackend {
    fn from(value: T) -> Self {
        Self(Box::new(value))
    }
}
