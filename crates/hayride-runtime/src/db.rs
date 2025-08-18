pub mod bindings;
pub mod db;
mod db_impl;

pub use db::DBCtx;
pub use db::{DBImpl, DBView};

use hayride_host_traits::db::DBTrait;

use wasmtime::component::HasData;

pub fn add_to_linker_sync<T>(l: &mut wasmtime::component::Linker<T>) -> anyhow::Result<()>
where
    T: DBView,
{
    crate::db::bindings::db::add_to_linker::<T, HasDB<T>>(l, |x| DBImpl(x))?;

    Ok(())
}

struct HasDB<T>(T);

impl<T: 'static> HasData for HasDB<T> {
    type Data<'a> = DBImpl<&'a mut T>;
}

pub struct DBBackend(Box<dyn DBTrait>);
impl std::ops::Deref for DBBackend {
    type Target = dyn DBTrait;
    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}
impl std::ops::DerefMut for DBBackend {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.as_mut()
    }
}
impl<T: DBTrait + 'static> From<T> for DBBackend {
    fn from(value: T) -> Self {
        Self(Box::new(value))
    }
}
