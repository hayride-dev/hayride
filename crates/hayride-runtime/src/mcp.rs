mod mcp_impl;

pub mod mcp;
pub mod bindings;

pub use mcp::McpCtx;
pub use mcp::{McpImpl, McpView};

use wasmtime::component::HasData;

pub fn add_to_linker_sync<T>(l: &mut wasmtime::component::Linker<T>) -> anyhow::Result<()>
where
    T: McpView,
{
    // Context, Tools, and Auth bindings are added as a fallback to satisfy the imports if they are needed.
    bindings::mcp::tools::add_to_linker::<T, HasMcp<T>>(l, |x| McpImpl(x))?;
    bindings::mcp::auth::add_to_linker::<T, HasMcp<T>>(l, |x| McpImpl(x))?;

    Ok(())
}

struct HasMcp<T>(T);

impl<T: 'static> HasData for HasMcp<T> {
    type Data<'a> = McpImpl<&'a mut T>;
}
