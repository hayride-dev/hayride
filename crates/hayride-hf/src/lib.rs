use anyhow::Result;

use hf_hub::api::sync::ApiBuilder;

use hayride_host_traits::ai::model::{
    ErrorCode, ModelLoaderInner
};

pub struct HuggingFaceModelLoader {
    api: hf_hub::api::sync::Api,
}

impl HuggingFaceModelLoader {
    pub fn new() -> Result<Self> {
        
        let hayride_dir = hayride_utils::paths::hayride::default_hayride_dir()?;
        let custom_cache = hayride_dir.join("ai/hf_hub");

        // Create the custom cache directory if it does not exist
        std::fs::create_dir_all(&custom_cache)?;

        // Build the API with the custom cache directory
        let api = ApiBuilder::new().with_cache_dir(custom_cache).build()?;

        Ok(HuggingFaceModelLoader {
            api: api,
        })
    }
}

impl ModelLoaderInner for HuggingFaceModelLoader {
    // Load a model from Hugging Face Hub
    // The name should be in the format "owner_name/repo_name/model_file"
    fn load(&mut self, name: String) -> Result<String, ErrorCode> {
        // Parse the model file from the repo id
        let parts: Vec<&str> = name.split('/').collect();

        // Ensure we have at least repo and model file
        if parts.len() < 2 {
            return Err(ErrorCode::InvalidModelName);
        }

        // If just 2 use the first part as the repo and the second as the model file, 
        // else recombine the owner and repo name for model_id
        let (model_id, model_file) = if parts.len() == 2 {
            (parts[0].to_string(), parts[1])
        } else {
            let repo_name = parts[..parts.len() - 1].join("/");
            (repo_name, parts[parts.len() - 1])
        };
        

        let model = self.api.model(model_id);
        let path = model.get(model_file).map_err(|err| {
            log::error!("Failed to get model file '{}': {}", model_file, err);
            ErrorCode::RuntimeError
        })?;

        Ok(path.to_string_lossy().to_string())
    }
}

