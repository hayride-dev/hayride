use std::path::PathBuf;

use anyhow::Result;

use hf_hub::api::sync::ApiBuilder;

use hayride_host_traits::ai::model::{ErrorCode, ModelRepositoryInner};

pub struct HuggingFaceModelRepository {
    api: hf_hub::api::sync::Api,
    cache: PathBuf,
}

impl HuggingFaceModelRepository {
    pub fn new() -> Result<Self> {
        let hayride_dir = hayride_utils::paths::hayride::default_hayride_dir()?;
        let custom_cache = hayride_dir.join("ai/hf_hub");

        // Create the custom cache directory if it does not exist
        std::fs::create_dir_all(&custom_cache)?;

        // Build the API with the custom cache directory
        let api = ApiBuilder::new()
            .with_cache_dir(custom_cache.clone())
            .with_progress(false) // Disable progress bar
            .build()?;

        Ok(HuggingFaceModelRepository { api: api, cache: custom_cache })
    }
}

impl ModelRepositoryInner for HuggingFaceModelRepository {
    // Download a model from Hugging Face Hub
    // The name should be in the format "owner_name/repo_name/model_file"
    fn download(&mut self, name: String) -> Result<String, ErrorCode> {
        // Parse the model file from the repo id
        let (model_id, model_file) = parse_model_name(&name)?;

        let model = self.api.model(model_id);
        let path = model.get(model_file).map_err(|err| {
            log::error!("Failed to get model file '{}': {}", model_file, err);
            ErrorCode::RuntimeError
        })?;

        Ok(path.to_string_lossy().to_string())
    }

    fn get(&self, name: String) -> Result<String, ErrorCode> {
        // Parse the model file from the repo id
        let (model_id, model_file) = parse_model_name(&name)?;

        // Use the cache to check if the model is already downloaded
        let repo = hf_hub::Repo::new(model_id, hf_hub::RepoType::Model);
        let cache = hf_hub::Cache::new(self.cache.clone());

        if let Some(path) = cache.repo(repo).get(model_file) {
            return Ok(path.to_string_lossy().to_string());
        }

        return Err(ErrorCode::ModelNotFound);
    }

    fn delete(&mut self, name: String) -> std::result::Result<(), ErrorCode> {
        let (model_id, model_file) = parse_model_name(&name)?;

        let repo = hf_hub::Repo::new(model_id, hf_hub::RepoType::Model);
        let cache = hf_hub::Cache::new(self.cache.clone());

        if let Some(path) = cache.repo(repo).get(model_file) {
            // Remove the file from the cache
            std::fs::remove_file(path).map_err(|_| ErrorCode::RuntimeError)?;
        }
        
        return Err(ErrorCode::ModelNotFound);
    }

    fn list(&self) -> std::result::Result<Vec<String>, ErrorCode> {
        // List all models in the cache directory
        let mut models = Vec::new();

        if let Ok(entries) = std::fs::read_dir(&self.cache) {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    // Only include files that look like model files
                    // Currently only support for gguf model files
                    if name.ends_with(".gguf") {
                        models.push(name.to_string());
                    }
                }
            }
        } else {
            return Err(ErrorCode::RuntimeError);
        }

        Ok(models)
    }
}

fn parse_model_name(name: &str) -> Result<(String, &str), ErrorCode> {
    let parts: Vec<&str> = name.split('/').collect();

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

    Ok((model_id, model_file))
}