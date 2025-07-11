use crate::wac::bindings::{types::ErrorCode, wac};
use crate::wac::{WacImpl, WacView};
use hayride_host_traits::wac::Error;

use wasmtime::component::Resource;
use wasmtime::Result;

use anyhow::anyhow;

impl<T> wac::Host for WacImpl<T>
where
    T: WacView,
{
    fn compose(
        &mut self,
        path: String,
    ) -> Result<Result<Vec<u8>, Resource<wac::Error>>, anyhow::Error> {
        let result = self.ctx().wac_backend.compose(path.clone());

        match result {
            Ok(c) => {
                return Ok(Ok(c));
            }
            Err(e) => {
                let error = Error {
                    code: e,
                    data: anyhow!("Error composing path: {}", path),
                };
                let id = self.table().push(error)?;
                return Ok(Err(id));
            }
        }
    }

    fn plug(
        &mut self,
        socket_path: String,
        plug_path: Vec<String>,
    ) -> Result<Result<Vec<u8>, Resource<wac::Error>>, anyhow::Error> {
        let result = self.ctx().wac_backend.plug(socket_path.clone(), plug_path);

        match result {
            Ok(c) => {
                return Ok(Ok(c));
            }
            Err(e) => {
                let error = Error {
                    code: e,
                    data: anyhow!("Error plugging socket path: {}", socket_path),
                };
                let id = self.table().push(error)?;
                return Ok(Err(id));
            }
        }
    }
}

impl<T> wac::HostError for WacImpl<T>
where
    T: WacView,
{
    fn code(&mut self, error: Resource<Error>) -> Result<ErrorCode> {
        let error = self.table().get(&error)?;
        match error.code {
            hayride_host_traits::wac::ErrorCode::FileNotFound => Ok(ErrorCode::FileNotFound),
            hayride_host_traits::wac::ErrorCode::ComposeFailed => Ok(ErrorCode::ComposeFailed),
            hayride_host_traits::wac::ErrorCode::ResolveFailed => Ok(ErrorCode::ResolveFailed),
            hayride_host_traits::wac::ErrorCode::EncodeFailed => Ok(ErrorCode::EncodeFailed),
            hayride_host_traits::wac::ErrorCode::Unknown => Ok(ErrorCode::Unknown),
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
