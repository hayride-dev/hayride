pub mod bindings;
pub mod silo;
mod silo_impl;

pub use silo::SiloCtx;
pub use silo::{SiloImpl, SiloView};

use wasmtime::component::HasData;

pub fn add_to_linker_sync<T>(l: &mut wasmtime::component::Linker<T>) -> anyhow::Result<()>
where
    T: SiloView,
{
    crate::silo::bindings::process::add_to_linker::<T, HasSilo<T>>(l, |x| SiloImpl(x))?;
    crate::silo::bindings::threads::add_to_linker::<T, HasSilo<T>>(l, |x| SiloImpl(x))?;

    Ok(())
}

struct HasSilo<T>(T);

impl<T: 'static> HasData for HasSilo<T> {
    type Data<'a> = SiloImpl<&'a mut T>;
}
