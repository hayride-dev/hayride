use super::bindings::mcp::{auth, tools};
use super::mcp::{McpImpl, McpView};

use hayride_host_traits::mcp::auth::{ErrorCode as AuthErrorCode, Provider};
use hayride_host_traits::mcp::tools::{ErrorCode as ToolsErrorCode, Tools};

use wasmtime::component::Resource;
use wasmtime::Result;

impl<T> tools::Host for McpImpl<T> where T: McpView {}

impl<T> tools::HostTools for McpImpl<T>
where
    T: McpView,
{
    fn new(&mut self) -> Result<Resource<Tools>> {
        let tools = Tools {};
        let id: Resource<Tools> = self.table().push(tools)?;
        Ok(id)
    }

    fn call_tool(
        &mut self,
        _self: Resource<Tools>,
        _params: tools::CallToolParams,
    ) -> Result<Result<tools::CallToolResult, Resource<hayride_host_traits::mcp::tools::Error>>>
    {
        let e = tools::Error {
            code: ToolsErrorCode::Unknown,
            data: anyhow::anyhow!("Tools not enabled").into(),
        };
        let r = self.table().push(e)?;
        return Ok(Err(r));
    }

    fn list_tools(
        &mut self,
        _self: Resource<Tools>,
        _cursor: String,
    ) -> Result<Result<tools::ListToolsResult, Resource<hayride_host_traits::mcp::tools::Error>>>
    {
        let e = tools::Error {
            code: ToolsErrorCode::Unknown,
            data: anyhow::anyhow!("Tools not enabled").into(),
        };
        let r = self.table().push(e)?;
        return Ok(Err(r));
    }

    fn drop(&mut self, id: Resource<Tools>) -> Result<()> {
        self.table().delete(id)?;
        Ok(())
    }
}

impl<T> tools::HostError for McpImpl<T>
where
    T: McpView,
{
    fn code(&mut self, error: Resource<tools::Error>) -> Result<tools::ErrorCode> {
        let error = self.table().get(&error)?;
        match error.code {
            ToolsErrorCode::ToolNotFound => Ok(tools::ErrorCode::ToolNotFound),
            ToolsErrorCode::ToolCallFailed => Ok(tools::ErrorCode::ToolCallFailed),
            ToolsErrorCode::Unknown => Ok(tools::ErrorCode::Unknown),
        }
    }

    fn data(&mut self, error: Resource<tools::Error>) -> Result<String> {
        let error = self.table().get(&error)?;
        return Ok(error.data.to_string());
    }

    fn drop(&mut self, error: Resource<tools::Error>) -> Result<()> {
        self.table().delete(error)?;
        return Ok(());
    }
}

impl<T> auth::Host for McpImpl<T> where T: McpView {}

impl<T> auth::HostProvider for McpImpl<T>
where
    T: McpView,
{
    fn new(&mut self) -> Result<Resource<Provider>> {
        let provider = auth::Provider {};
        let id: Resource<auth::Provider> = self.table().push(provider)?;
        Ok(id)
    }

    fn auth_url(
        &mut self,
        _self: Resource<Provider>,
    ) -> Result<Result<String, Resource<auth::Error>>> {
        let e = auth::Error {
            code: AuthErrorCode::Unknown,
            data: anyhow::anyhow!("Auth not implemented").into(),
        };
        let r = self.table().push(e)?;
        return Ok(Err(r));
    }

    fn registration(
        &mut self,
        _self: Resource<Provider>,
        _data: Vec<u8>,
    ) -> Result<Result<Vec<u8>, Resource<auth::Error>>> {
        let e = auth::Error {
            code: AuthErrorCode::Unknown,
            data: anyhow::anyhow!("Auth not implemented").into(),
        };
        let r = self.table().push(e)?;
        return Ok(Err(r));
    }

    fn exchange_code(
        &mut self,
        _self: Resource<Provider>,
        _data: Vec<u8>,
    ) -> Result<Result<Vec<u8>, Resource<auth::Error>>> {
        let e = auth::Error {
            code: AuthErrorCode::Unknown,
            data: anyhow::anyhow!("Auth not implemented").into(),
        };
        let r = self.table().push(e)?;
        return Ok(Err(r));
    }

    fn validate(
        &mut self,
        _self: Resource<Provider>,
        _token: String,
    ) -> Result<Result<bool, Resource<auth::Error>>> {
        let e = auth::Error {
            code: AuthErrorCode::Unknown,
            data: anyhow::anyhow!("Auth not implemented").into(),
        };
        let r = self.table().push(e)?;
        return Ok(Err(r));
    }

    fn drop(&mut self, id: Resource<auth::Provider>) -> Result<()> {
        self.table().delete(id)?;
        Ok(())
    }
}

impl<T> auth::HostError for McpImpl<T>
where
    T: McpView,
{
    fn code(&mut self, error: Resource<auth::Error>) -> Result<auth::ErrorCode> {
        let error = self.table().get(&error)?;
        match error.code {
            AuthErrorCode::AuthUrlFailed => Ok(auth::ErrorCode::AuthUrlFailed),
            AuthErrorCode::RegistrationFailed => Ok(auth::ErrorCode::RegistrationFailed),
            AuthErrorCode::ExchangeCodeFailed => Ok(auth::ErrorCode::ExchangeCodeFailed),
            AuthErrorCode::ValidateFailed => Ok(auth::ErrorCode::ValidateFailed),
            AuthErrorCode::Unknown => Ok(auth::ErrorCode::Unknown),
        }
    }

    fn data(&mut self, error: Resource<auth::Error>) -> Result<String> {
        let error = self.table().get(&error)?;
        return Ok(error.data.to_string());
    }

    fn drop(&mut self, error: Resource<auth::Error>) -> Result<()> {
        self.table().delete(error)?;
        return Ok(());
    }
}
