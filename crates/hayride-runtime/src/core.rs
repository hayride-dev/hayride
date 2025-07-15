pub mod bindings;
pub mod core;
mod core_impl;

pub use core::CoreCtx;
pub use core::{CoreImpl, CoreView};

use hayride_host_traits::core::version::VersionInner;

use wasmtime::component::HasData;

pub fn add_to_linker_sync<T>(l: &mut wasmtime::component::Linker<T>) -> anyhow::Result<()>
where
    T: CoreView,
{
    crate::core::bindings::version::add_to_linker::<T, HasCore<T>>(l, |x| CoreImpl(x))?;

    Ok(())
}

struct HasCore<T>(T);

impl<T: 'static> HasData for HasCore<T> {
    type Data<'a> = CoreImpl<&'a mut T>;
}

pub struct VersionBackend(Box<dyn VersionInner>);
impl std::ops::Deref for VersionBackend {
    type Target = dyn VersionInner;
    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}
impl std::ops::DerefMut for VersionBackend {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.as_mut()
    }
}
impl<T: VersionInner + 'static> From<T> for VersionBackend {
    fn from(value: T) -> Self {
        Self(Box::new(value))
    }
}
