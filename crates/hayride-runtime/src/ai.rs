mod ai_impl;

pub mod ai;
pub mod bindings;

pub use ai::AiCtx;
pub use ai::{AiImpl, AiView};

pub use bindings::inference::GraphExecutionContext;

use hayride_host_traits::ai::rag::RagInner;
use hayride_host_traits::ai::BackendInner;

pub fn add_to_linker_sync<T>(l: &mut wasmtime::component::Linker<T>) -> anyhow::Result<()>
where
    T: AiView,
{
    let closure = type_annotate_ml::<T, _>(|t| AiImpl(t));
    bindings::tensor::add_to_linker_get_host(l, closure)?;
    bindings::graph::add_to_linker_get_host(l, closure)?;
    bindings::inference::add_to_linker_get_host(l, closure)?;
    bindings::errors::add_to_linker_get_host(l, closure)?;
    bindings::tensor_stream::add_to_linker_get_host(l, closure)?;
    bindings::graph_stream::add_to_linker_get_host(l, closure)?;
    bindings::inference_stream::add_to_linker_get_host(l, closure)?;
    bindings::rag::add_to_linker_get_host(l, closure)?;
    bindings::transformer::add_to_linker_get_host(l, closure)?;

    Ok(())
}

// NB: workaround some rustc inference - a future refactoring may make this
// obsolete.
fn type_annotate_ml<T, F>(val: F) -> F
where
    F: Fn(&mut T) -> AiImpl<&mut T>,
{
    val
}

/// A machine learning backend.
pub struct Backend(Box<dyn BackendInner>);
impl std::ops::Deref for Backend {
    type Target = dyn BackendInner;
    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}
impl std::ops::DerefMut for Backend {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.as_mut()
    }
}
impl<T: BackendInner + 'static> From<T> for Backend {
    fn from(value: T) -> Self {
        Self(Box::new(value))
    }
}

/// A rag backend
pub struct Rag(Box<dyn RagInner>);
impl std::ops::Deref for Rag {
    type Target = dyn RagInner;
    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}
impl std::ops::DerefMut for Rag {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.as_mut()
    }
}
impl<T: RagInner + 'static> From<T> for Rag {
    fn from(value: T) -> Self {
        Self(Box::new(value))
    }
}
