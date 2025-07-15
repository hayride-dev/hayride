use crate::core::bindings::{version, version::ErrorCode};
use crate::core::{CoreImpl, CoreView};
use hayride_host_traits::core::version::Error;

use wasmtime::component::Resource;
use wasmtime::Result;

use anyhow::anyhow;
use std::time::{SystemTime, UNIX_EPOCH};

impl<T> version::Host for CoreImpl<T>
where
    T: CoreView,
{
    fn latest(&mut self) -> Result<Result<String, Resource<version::Error>>> {
        let ctx = self.ctx();
        let cache = ctx.get_version_cache();
        let now = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e| {
            anyhow!("Error getting current time: {}", e)
        })?.as_secs();

        // Only check the version if it's been more than an hour since the last check
        let should_check = match cache.last_check {
            Some(last) => now > last + 3600,
            None => true,
        };

        if !should_check {
            // If we have a cached version and it's still valid, return it
            if let Some(version) = cache.last_version {
                return Ok(Ok(version));
            }
        }

        let result = ctx.version_backend.latest();
        match result {
            Ok(version) => {
                // Store the new version in the cache
                ctx.set_version_cache(Some(now), Some(version.clone()));
                Ok(Ok(version))
            },
            Err(e) => {
                let error = Error {
                    code: e,
                    data: anyhow!("Error retrieving latest version"),
                };
                let id = self.table().push(error)?;
                Ok(Err(id))
            }
        }
    }
}

impl<T> version::HostError for CoreImpl<T>
where
    T: CoreView,
{
    fn code(&mut self, error: Resource<Error>) -> Result<ErrorCode> {
        let error = self.table().get(&error)?;
        match error.code {
            hayride_host_traits::core::version::ErrorCode::GetVersionFailed => {
                Ok(ErrorCode::GetVersionFailed)
            }
            hayride_host_traits::core::version::ErrorCode::Unknown => Ok(ErrorCode::Unknown),
        }
    }

    fn data(&mut self, error: Resource<Error>) -> Result<String> {
        let error = self.table().get(&error)?;
        return Ok(error.data.to_string());
    }

    fn drop(&mut self, error: Resource<Error>) -> Result<()> {
        self.table().delete(error)?;
        return Ok(());
    }
}
