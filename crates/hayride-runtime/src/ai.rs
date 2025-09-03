mod ai_impl;

pub mod ai;
pub mod bindings;

pub use ai::AiCtx;
pub use ai::{AiImpl, AiView};

pub use bindings::inference::GraphExecutionContext;

use hayride_host_traits::ai::model::ModelRepositoryInner;
use hayride_host_traits::ai::rag::RagInner;
use hayride_host_traits::ai::BackendInner;

use wasmtime::component::HasData;

pub fn add_to_linker_sync<T>(l: &mut wasmtime::component::Linker<T>) -> anyhow::Result<()>
where
    T: AiView,
{
    bindings::tensor::add_to_linker::<T, HasAi<T>>(l, |x| AiImpl(x))?;
    bindings::graph::add_to_linker::<T, HasAi<T>>(l, |x| AiImpl(x))?;
    bindings::inference::add_to_linker::<T, HasAi<T>>(l, |x| AiImpl(x))?;
    bindings::errors::add_to_linker::<T, HasAi<T>>(l, |x| AiImpl(x))?;
    bindings::ai::tensor_stream::add_to_linker::<T, HasAi<T>>(l, |x| AiImpl(x))?;
    bindings::ai::graph_stream::add_to_linker::<T, HasAi<T>>(l, |x| AiImpl(x))?;
    bindings::ai::inference_stream::add_to_linker::<T, HasAi<T>>(l, |x| AiImpl(x))?;
    bindings::ai::rag::add_to_linker::<T, HasAi<T>>(l, |x| AiImpl(x))?;
    bindings::ai::transformer::add_to_linker::<T, HasAi<T>>(l, |x| AiImpl(x))?;
    bindings::ai::model_repository::add_to_linker::<T, HasAi<T>>(l, |x| AiImpl(x))?;

    // Context added as a fallback to satisfy the imports if needed.
    bindings::ai::context::add_to_linker::<T, HasAi<T>>(l, |x| AiImpl(x))?;

    Ok(())
}

struct HasAi<T>(T);

impl<T: 'static> HasData for HasAi<T> {
    type Data<'a> = AiImpl<&'a mut T>;
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

// ModelRepository backend
pub struct ModelRepository(Box<dyn ModelRepositoryInner>);
impl std::ops::Deref for ModelRepository {
    type Target = dyn ModelRepositoryInner;
    fn deref(&self) -> &Self::Target {
        self.0.as_ref()
    }
}
impl std::ops::DerefMut for ModelRepository {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0.as_mut()
    }
}
impl<T: ModelRepositoryInner + 'static> From<T> for ModelRepository {
    fn from(value: T) -> Self {
        Self(Box::new(value))
    }
}
