pub mod bindings;
pub mod wac;
mod wac_impl;

pub use wac::WacCtx;
pub use wac::{WacImpl, WacView};

use hayride_host_traits::wac::WacTrait;

use wasmtime::component::HasData;

pub fn add_to_linker_sync<T>(l: &mut wasmtime::component::Linker<T>) -> anyhow::Result<()>
where
    T: WacView,
{
    crate::wac::bindings::wac::add_to_linker::<T, HasWac<T>>(l, |x| WacImpl(x))?;

    Ok(())
}

struct HasWac<T>(T);

impl<T: 'static> HasData for HasWac<T> {
    type Data<'a> = WacImpl<&'a mut T>;
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
