use std::collections::HashMap;
use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::ptr::NonNull;

use rand::Rng;
use serde::{Deserialize, Serialize};
use tokio::io::{self, AsyncWriteExt, DuplexStream};
use tokio::runtime::Runtime;
use tokio::task::block_in_place;

use hayride_host_traits::ai::{
    BackendError, BackendExecutionContext, BackendGraph, BackendInner, ExecutionContext, Graph,
    Tensor, TensorStream, TensorType,
};

#[derive(Serialize, Deserialize)]
pub struct PromptOptions {
    temperature: f32,
    num_context: i32,
    num_batch: i32,
    max_predict: i32,
    top_k: i32,
    top_p: f32,
    seed: u32,
}

// RAII wrapper for llama context to ensure proper cleanup
struct LlamaContextGuard {
    context: *mut hayride_llama_rs_sys::llama_context,
}

impl LlamaContextGuard {
    fn new(context: *mut hayride_llama_rs_sys::llama_context) -> Option<Self> {
        if context.is_null() {
            None
        } else {
            Some(Self { context })
        }
    }

    fn as_ptr(&self) -> *mut hayride_llama_rs_sys::llama_context {
        self.context
    }

    // Clear the KV cache to free up memory and reset context state
    fn clear_kv_cache(&self) {
        if !self.context.is_null() {
            unsafe {
                hayride_llama_rs_sys::llama_kv_self_clear(self.context);
            }
            log::debug!("Cleared KV cache to free memory");
        }
    }
}

impl Drop for LlamaContextGuard {
    fn drop(&mut self) {
        if !self.context.is_null() {
            log::debug!("freeing llama context");
            unsafe {
                hayride_llama_rs_sys::llama_free(self.context);
            }
        }
    }
}

// RAII wrapper for llama sampler to ensure proper cleanup
struct LlamaSamplerGuard {
    sampler: *mut hayride_llama_rs_sys::llama_sampler,
}

impl LlamaSamplerGuard {
    fn new(sampler: *mut hayride_llama_rs_sys::llama_sampler) -> Option<Self> {
        if sampler.is_null() {
            None
        } else {
            Some(Self { sampler })
        }
    }

    fn as_ptr(&self) -> *mut hayride_llama_rs_sys::llama_sampler {
        self.sampler
    }
}

impl Drop for LlamaSamplerGuard {
    fn drop(&mut self) {
        if !self.sampler.is_null() {
            log::debug!("freeing llama sampler");
            unsafe {
                hayride_llama_rs_sys::llama_sampler_free(self.sampler);
            }
        }
    }
}

#[derive(Default)]
pub struct LlamaCppBackend {
    models: HashMap<String, NonNull<hayride_llama_rs_sys::llama_model>>,
}

unsafe impl Send for LlamaCppBackend {}
unsafe impl Sync for LlamaCppBackend {}

impl LlamaCppBackend {
    pub fn new() -> Self {
        unsafe {
            hayride_llama_rs_sys::llama_backend_init();
            hayride_llama_rs_sys::llama_log_set(Some(llama_log_callback), std::ptr::null_mut());
        }

        LlamaCppBackend {
            models: HashMap::new(),
        }
    }
}

unsafe extern "C" fn llama_log_callback(
    level: hayride_llama_rs_sys::ggml_log_level,
    text: *const c_char,
    _user_data: *mut c_void,
) {
    let text = unsafe {
        // SAFETY: `text` is a NUL-terminated C String.
        CStr::from_ptr(text)
    };
    let text = String::from_utf8_lossy(text.to_bytes());

    // Skip empty log messages
    if text.len() < 2 {
        return;
    }

    let text = if let Some(stripped) = text.strip_suffix('\n') {
        stripped
    } else {
        text.as_ref()
    };

    // TODO: Allow setting custom log level for llama.cpp
    // currently using log level that matches the env
    match level {
        hayride_llama_rs_sys::GGML_LOG_LEVEL_DEBUG => log::debug!("{}", text),
        hayride_llama_rs_sys::GGML_LOG_LEVEL_INFO => log::info!("{}", text),
        hayride_llama_rs_sys::GGML_LOG_LEVEL_WARN => log::warn!("{}", text),
        hayride_llama_rs_sys::GGML_LOG_LEVEL_ERROR => log::error!("{}", text),
        _ => unimplemented!(),
    }
}

impl Drop for LlamaCppBackend {
    fn drop(&mut self) {
        // Free all loaded models first
        for (name, model) in self.models.drain() {
            log::debug!("freeing model: {}", name);
            unsafe {
                hayride_llama_rs_sys::llama_free_model(model.as_ptr());
            }
        }

        unsafe {
            // SAFETY: This is only called when no models or sessions exist.
            hayride_llama_rs_sys::llama_backend_free();
        }
    }
}

impl BackendInner for LlamaCppBackend {
    fn load(&mut self, name: String) -> Result<Graph, BackendError> {
        log::debug!("loading LlamaCpp model: {}", name);

        if let Some(model) = self.models.get(&name) {
            let graph: Box<dyn BackendGraph> = Box::new(LlamaCppGraph { model: *model });
            return Ok(graph.into());
        }

        let cstr = CString::new(name.clone()).map_err(|_| BackendError::FailedToLoadModel)?;
        let model: NonNull<hayride_llama_rs_sys::llama_model>;
        unsafe {
            // TODO: Set model parameters
            let params = hayride_llama_rs_sys::llama_model_default_params();
            // params.n_gpu_layers = 81;
            log::debug!("model params: {:?}", params);

            // Load the model here
            let llama_model: *mut hayride_llama_rs_sys::llama_model =
                hayride_llama_rs_sys::llama_load_model_from_file(cstr.as_ptr(), params);
            if llama_model.is_null() {
                return Err(BackendError::FailedToLoadModel);
            }

            log::debug!("model: {:?}", llama_model);

            model = NonNull::new(llama_model).ok_or(BackendError::FailedToLoadModel)?;
        }

        self.models.insert(name.clone(), model);
        let graph: Box<dyn BackendGraph> = Box::new(LlamaCppGraph { model: model });
        Ok(graph.into())
    }
}

struct LlamaCppGraph {
    model: NonNull<hayride_llama_rs_sys::llama_model>,
}

// Needed because NonNull pointer is not Send/Sync
unsafe impl Send for LlamaCppGraph {}
unsafe impl Sync for LlamaCppGraph {}

impl LlamaCppGraph {
    fn get_model(&self) -> NonNull<hayride_llama_rs_sys::llama_model> {
        self.model
    }
}

impl Drop for LlamaCppGraph {
    fn drop(&mut self) {
        log::debug!("dropping LlamaCppGraph");
        // Note: We don't free the model here as it's managed by LlamaCppBackend
        // The model will be freed when the backend is dropped
    }
}

impl BackendGraph for LlamaCppGraph {
    fn init_execution_context(&self) -> Result<ExecutionContext, BackendError> {
        let context: Box<dyn BackendExecutionContext> =
            Box::new(LlamaCppExecutionContext { model: self.model });
        return Ok(context.into());
    }
}

struct LlamaCppExecutionContext {
    model: NonNull<hayride_llama_rs_sys::llama_model>,
}

// Needed because NonNull pointer is not Send/Sync
unsafe impl Send for LlamaCppExecutionContext {}
unsafe impl Sync for LlamaCppExecutionContext {}

impl Drop for LlamaCppExecutionContext {
    fn drop(&mut self) {
        log::debug!("dropping LlamaCppExecutionContext");
        // Note: We don't free the model here as it's managed by LlamaCppBackend
        // The model will be freed when the backend is dropped
    }
}

impl BackendExecutionContext for LlamaCppExecutionContext {
    fn compute(&mut self, tensors: Vec<(String, Tensor)>) -> Result<Tensor, BackendError> {
        let graph = LlamaCppGraph { model: self.model };
        let mut options_tensor = None;
        let mut input_tensor = None;
        for (id, tensor) in tensors {
            if id == "options" {
                options_tensor = Some(tensor);
            } else {
                input_tensor = Some(tensor);
            }
        }

        let input_tensor: Tensor = input_tensor
            .clone()
            .ok_or(BackendError::FailedTensorNotSet)?;

        // Validate input size before processing to prevent memory issues
        if input_tensor.data.len() > 1_000_000 {
            // 1MB limit
            log::warn!(
                "Input tensor size ({} bytes) is very large, this may cause memory issues",
                input_tensor.data.len()
            );
        }

        let mut result = process_compute(graph, input_tensor, options_tensor, None)?;

        // Trim whitespace off of result
        result = result.trim().to_string();
        log::debug!("setting result tensor with data: [{}]", result);

        // Build result tensor manually
        let result_tensor = Tensor {
            data: result.as_bytes().to_vec(),
            dimensions: vec![1],
            ty: TensorType::U8,
        };

        Ok(result_tensor)
    }

    fn compute_stream(
        &mut self,
        tensors: Vec<(String, Tensor)>,
    ) -> Result<TensorStream, BackendError> {
        // Use duplex writer/reader for the async stream
        let (writer, reader) = io::duplex(4096);

        let graph = LlamaCppGraph { model: self.model };
        let mut options_tensor = None;
        let mut input_tensor = None;
        for (id, tensor) in tensors {
            if id == "options" {
                options_tensor = Some(tensor);
            } else {
                input_tensor = Some(tensor);
            }
        }

        let input_tensor: Tensor = input_tensor
            .clone()
            .ok_or(BackendError::FailedTensorNotSet)?;

        tokio::task::spawn(async move {
            // Provide writer for async compute
            let result = process_compute(graph, input_tensor, options_tensor, Some(writer));
            if let Err(e) = result {
                log::warn!("error in compute_stream: {:?}", e);
            }
        });

        let tensor = TensorStream::new(vec![1], TensorType::U8, reader);

        Ok(tensor)
    }
}

fn process_compute(
    graph: LlamaCppGraph,
    input: Tensor,
    options: Option<Tensor>,
    mut writer: Option<DuplexStream>,
) -> Result<String, BackendError> {
    let start = std::time::Instant::now();
    let llama_model = graph.get_model();
    let llama_vocab = unsafe { hayride_llama_rs_sys::llama_model_get_vocab(llama_model.as_ptr()) };

    // Check for options and override defaults if set
    let max_context = 128000; // 128k max context for llama.cpp
    let mut num_context = 40960;
    let mut batch_size: i32 = 2048;
    let mut max_predict = 5000;
    let mut temperature = 0.8; // Default to greedy
    let mut top_k = 20;
    let mut top_p = 0.95;
    let penalty_last_n = 512;
    let penalty_repeat = 1.25;
    let penalty_frequency = 0.5;
    let penalty_presence = 0.5;
    let mut rng = rand::rng(); // Default random seed
    let mut seed: u32 = rng.random();
    match options {
        Some(tensor) => {
            let options_str =
                String::from_utf8(tensor.data.clone()).map_err(|_| BackendError::FailedDecoding)?;
            let options: PromptOptions =
                serde_json::from_str(&options_str).map_err(|_| BackendError::FailedDecoding)?;
            if options.num_context != 0 {
                num_context = options.num_context;

                if options.num_context > max_context {
                    num_context = max_context;
                }
            }
            if options.num_batch != 0 {
                batch_size = options.num_batch;

                if options.num_batch > num_context {
                    batch_size = num_context;
                }
            }
            if options.max_predict != 0 {
                max_predict = options.max_predict;
            }
            if options.top_k != 0 {
                top_k = options.top_k;
            }
            if options.seed != 0 {
                seed = options.seed;
            }

            temperature = options.temperature;
            top_p = options.top_p;
        }
        None => {}
    }

    let mut context_params: hayride_llama_rs_sys::llama_context_params =
        unsafe { hayride_llama_rs_sys::llama_context_default_params() };
    context_params.n_batch = batch_size as u32; // size of the logits and embeddings buffer, which limits the maximum batch size passed to llama_decode
    context_params.n_ctx = num_context as u32; // The context size is the maximum number of tokens that the model can account for when processing a response
    context_params.n_ubatch = 512; // physical maximum batch size for computation batch_size >= ubatch_size
                                   // context_params.n_threads = 8; // number of threads to use for computation
    log::debug!("context params: {:?}", context_params);

    // Create context
    let llama_context_ptr: *mut hayride_llama_rs_sys::llama_context = unsafe {
        hayride_llama_rs_sys::llama_new_context_with_model(llama_model.as_ptr(), context_params)
    };

    // Use RAII wrapper to ensure cleanup
    let mut llama_context = LlamaContextGuard::new(llama_context_ptr).ok_or_else(|| {
        let error_msg = "Failed to create llama context - possibly out of memory";
        log::error!("{}", error_msg);
        BackendError::FailedToLoadModel
    })?;

    // Tokenize the prompt
    let prompt: Vec<u8> = input.data.clone();
    // convert prompt to string
    let prompt_str = String::from_utf8(prompt).map_err(|_| BackendError::FailedTokenization);
    let prompt_str = match prompt_str {
        Ok(s) => s,
        Err(e) => {
            // If Writer set, write error to the buffer, blocking while we write to the stream
            if let Some(writer) = writer {
                write_output(writer, &e.to_string())?;
            }
            return Err(e);
        }
    };

    log::debug!("tokenizing prompt: {}", prompt_str);

    // find the number of tokens in the prompt
    let c_string = CString::new(prompt_str).map_err(|_| BackendError::FailedTokenization)?;
    let n_prompt = unsafe {
        -hayride_llama_rs_sys::llama_tokenize(
            llama_vocab,
            c_string.as_ptr(),
            c_int::try_from(c_string.as_bytes().len())
                .map_err(|_| BackendError::FailedTokenization)?,
            std::ptr::null_mut(),
            0,
            true, // Add the BOT and EOT token
            true, // Tokenize control tokens
        )
    };

    // Allocate space for the tokens and tokenize
    let mut prompt_tokens = Vec::with_capacity(
        n_prompt
            .try_into()
            .map_err(|_| BackendError::FailedTokenization)?,
    );
    let buffer_capacity =
        c_int::try_from(prompt_tokens.capacity()).expect("buffer capacity should fit into a c_int");

    let prompt_size = unsafe {
        hayride_llama_rs_sys::llama_tokenize(
            llama_vocab,
            c_string.as_ptr(),
            c_int::try_from(c_string.as_bytes().len())
                .map_err(|_| BackendError::FailedTokenization)?,
            prompt_tokens.as_mut_ptr(),
            buffer_capacity,
            true, // Add the BOT and EOT token
            true, // Tokenize control tokens
        )
    };
    if prompt_size < 0 {
        // If Writer set, write error to the buffer, blocking while we write to the stream
        if let Some(writer) = writer {
            write_output(writer, &BackendError::FailedTokenization.to_string())?;
        }
        return Err(BackendError::FailedTokenization);
    }

    // Handle context too large by dynamically adjusting batch size or truncating prompt
    if prompt_size >= batch_size {
        log::warn!(
            "Prompt size ({}) exceeds batch size ({}), attempting to handle...",
            prompt_size,
            batch_size
        );

        // Strategy 1: Try to increase batch size if within context limits
        let new_batch_size = std::cmp::min(prompt_size + 512, num_context);
        if new_batch_size <= num_context && new_batch_size > batch_size {
            log::info!(
                "Increasing batch size from {} to {} to accommodate prompt",
                batch_size,
                new_batch_size
            );
            batch_size = new_batch_size;

            // Recreate context with new batch size
            context_params.n_batch = batch_size as u32;

            // Drop the old context and create a new one
            drop(llama_context);
            let new_llama_context_ptr: *mut hayride_llama_rs_sys::llama_context = unsafe {
                hayride_llama_rs_sys::llama_new_context_with_model(
                    llama_model.as_ptr(),
                    context_params,
                )
            };

            llama_context = LlamaContextGuard::new(new_llama_context_ptr).ok_or_else(|| {
                let error_msg = "Failed to recreate llama context with larger batch size";
                log::error!("{}", error_msg);
                BackendError::FailedToLoadModel
            })?;
        } else {
            // Strategy 2: Truncate the prompt to fit within batch size
            let max_prompt_tokens = batch_size - 64; // Leave some room for generation
            log::warn!(
                "Truncating prompt from {} tokens to {} tokens",
                prompt_size,
                max_prompt_tokens
            );

            // Truncate from the beginning, keeping the end of the prompt
            let truncate_amount = prompt_size - max_prompt_tokens;
            prompt_tokens.drain(0..truncate_amount as usize);

            log::info!("Prompt truncated, new size: {} tokens", prompt_tokens.len());
        }
    }

    let size = usize::try_from(prompt_size).expect("size is positive and usize ");
    // Safety: `size` < `capacity` and llama-cpp has initialized elements up to `size`
    unsafe { prompt_tokens.set_len(size) }

    // initialize the sampler
    // https://github.com/ggerganov/llama.cpp/blob/master/examples/simple/simple.cpp#L118

    let mut sampler_params = unsafe { hayride_llama_rs_sys::llama_sampler_chain_default_params() };
    sampler_params.no_perf = false;
    let llama_sampler_ptr =
        unsafe { hayride_llama_rs_sys::llama_sampler_chain_init(sampler_params) };

    // Use RAII wrapper to ensure cleanup
    let llama_sampler = LlamaSamplerGuard::new(llama_sampler_ptr).ok_or_else(|| {
        log::error!("Failed to create llama sampler");
        BackendError::FailedToLoadModel
    })?;
    unsafe {
        // Add sampler params for temp
        if temperature > 0.0 {
            hayride_llama_rs_sys::llama_sampler_chain_add(
                llama_sampler.as_ptr(),
                hayride_llama_rs_sys::llama_sampler_init_top_k(top_k),
            );
            hayride_llama_rs_sys::llama_sampler_chain_add(
                llama_sampler.as_ptr(),
                hayride_llama_rs_sys::llama_sampler_init_top_p(top_p, 1),
            );
            hayride_llama_rs_sys::llama_sampler_init_penalties(
                penalty_last_n,
                penalty_repeat,
                penalty_frequency,
                penalty_presence,
            );
            hayride_llama_rs_sys::llama_sampler_chain_add(
                llama_sampler.as_ptr(),
                hayride_llama_rs_sys::llama_sampler_init_temp(temperature),
            );
            hayride_llama_rs_sys::llama_sampler_chain_add(
                llama_sampler.as_ptr(),
                hayride_llama_rs_sys::llama_sampler_init_dist(seed),
            );
        } else {
            // Temp of 0 uses greedy sampler
            hayride_llama_rs_sys::llama_sampler_chain_add(
                llama_sampler.as_ptr(),
                hayride_llama_rs_sys::llama_sampler_init_greedy(),
            );
        }
    }

    log::debug!("final prompt context size: {}", prompt_tokens.len());

    // prepare a batch for the prompt (use actual length after potential truncation)
    let mut batch = LlamaBatch::new(prompt_tokens.len());

    // Add tokens to batch
    let last_index: i32 = (prompt_tokens.len() - 1) as i32;
    for (i, token) in (0_i32..).zip(prompt_tokens.iter()) {
        let is_last = i == last_index;
        match batch.add(*token, i, &[0], is_last) {
            Ok(_) => {}
            Err(e) => {
                // If Writer set, write error to the buffer, blocking while we write to the stream
                if let Some(writer) = writer {
                    write_output(writer, &e.to_string())?;
                }
                return Err(e);
            }
        }
    }

    // main loop

    let start_time = unsafe { hayride_llama_rs_sys::ggml_time_us() };
    let mut n_decoded = 0;

    let mut position = 0;
    let mut result: String = "".to_owned();
    let actual_prompt_size = prompt_tokens.len() as i32;

    while position + batch.n_tokens() < actual_prompt_size + max_predict {
        // Check if we're approaching context limits and need to manage memory
        if position > num_context - 1000 {
            // Leave 1000 tokens buffer
            log::warn!(
                "Approaching context limit ({}), stopping generation early",
                num_context
            );
            break;
        }

        // evaluate the current batch with the transformer
        let res =
            unsafe { hayride_llama_rs_sys::llama_decode(llama_context.as_ptr(), batch.batch()) };
        if res != 0 {
            // Handle different decode error types
            match res {
                -3 => {
                    log::warn!("llama_decode failed with error -3 (likely memory/context issue), attempting recovery");
                    // Try clearing KV cache and retrying once
                    llama_context.clear_kv_cache();
                    let retry_res = unsafe {
                        hayride_llama_rs_sys::llama_decode(llama_context.as_ptr(), batch.batch())
                    };
                    if retry_res != 0 {
                        let error_msg = format!(
                            "llama_decode failed even after cache clear, error: {}",
                            retry_res
                        );
                        log::error!("{}", error_msg);
                        if let Some(writer) = writer {
                            write_output(writer, &error_msg)?;
                        }
                        return Err(BackendError::FailedTokenization);
                    } else {
                        log::info!("llama_decode succeeded after cache clear");
                    }
                }
                _ => {
                    let error_msg = format!("llama_decode failed with error: {}", res);
                    log::error!("{}", error_msg);
                    if let Some(writer) = writer {
                        write_output(writer, &error_msg)?;
                    }
                    return Err(BackendError::FailedTokenization);
                }
            }
        }

        position += batch.n_tokens();

        // sample the next token
        {
            let new_token_id = unsafe {
                hayride_llama_rs_sys::llama_sampler_sample(
                    llama_sampler.as_ptr(),
                    llama_context.as_ptr(),
                    -1,
                )
            };

            // is it and end of generation?
            if unsafe { hayride_llama_rs_sys::llama_token_is_eog(llama_vocab, new_token_id) } {
                break;
            }

            let string = CString::new(vec![b'*'; 32]).expect("no null");
            let len = string.as_bytes().len();
            let len = c_int::try_from(len).expect("length fits into c_int");
            let buf = string.into_raw();
            let n = unsafe {
                hayride_llama_rs_sys::llama_token_to_piece(
                    llama_vocab,
                    new_token_id,
                    buf,
                    len,
                    0,
                    true,
                )
            };
            if n < 0 {
                log::warn!("failed to convert token to piece");
                // If Writer set, write error to the buffer, blocking while we write to the stream
                if let Some(writer) = writer {
                    write_output(writer, &BackendError::FailedTokenization.to_string())?;
                }
                return Err(BackendError::FailedTokenization);
            }
            let string = unsafe { CString::from_raw(buf) };
            let mut bytes = string.into_bytes();
            let len = usize::try_from(n).expect("size is positive and fits into usize");
            bytes.truncate(len);
            // convert bytes to string
            let output = String::from_utf8(bytes).map_err(|_| BackendError::FailedTokenization);
            let output = match output {
                Ok(s) => s,
                Err(e) => {
                    // If Writer set, write error to the buffer, blocking while we write to the stream
                    if let Some(writer) = writer {
                        write_output(writer, &e.to_string())?;
                    }
                    return Err(e);
                }
            };

            // If Writer set, Write to the buffer, blocking while we write to the stream
            if let Some(ref mut writer) = writer {
                write_output(writer, &output)?;
            }

            // Push output for result
            result.push_str(&output);

            // prepare the next batch with the sampled token
            batch.clear();
            match batch.add(new_token_id, position, &[0], true) {
                Ok(_) => {}
                Err(e) => {
                    // If Writer set, write error to the buffer, blocking while we write to the stream
                    if let Some(writer) = writer {
                        write_output(writer, &e.to_string())?;
                    }
                    return Err(e);
                }
            }

            // Proactive context management: clear KV cache periodically to prevent memory buildup
            if n_decoded % 100 == 0 && position > num_context / 2 {
                log::debug!(
                    "Performing proactive KV cache cleanup at position {}",
                    position
                );
                llama_context.clear_kv_cache();

                // Reset position to prevent overflow
                position = actual_prompt_size;
                log::debug!("Reset position to {} after cache clear", position);
            }

            n_decoded += 1;
        }
    }

    let end_time = unsafe { hayride_llama_rs_sys::ggml_time_us() };

    let duration = start.elapsed();

    log::info!(
        "decoded {} tokens in {} s, total compute time: {} ms",
        n_decoded,
        (end_time - start_time) / 1000000,
        duration.as_millis()
    );

    // RAII wrappers will automatically free the sampler and context when they go out of scope

    return Ok(result);
}

pub struct LlamaBatch {
    allocated: usize,
    initialized_logits: Vec<i32>,
    llama_batch: hayride_llama_rs_sys::llama_batch,
}

impl LlamaBatch {
    pub fn new(n_tokens: usize) -> Self {
        let n_tokens_i32 = i32::try_from(n_tokens).expect("cannot fit n_tokens into a i32");
        let batch: hayride_llama_rs_sys::llama_batch =
            unsafe { hayride_llama_rs_sys::llama_batch_init(n_tokens_i32, 0, 1) };

        LlamaBatch {
            allocated: n_tokens,
            initialized_logits: vec![],
            llama_batch: batch,
        }
    }

    pub fn add(
        &mut self,
        token: i32,
        pos: i32,
        seq_ids: &[i32],
        logits: bool,
    ) -> Result<(), BackendError> {
        if self.allocated
            < usize::try_from(self.llama_batch.n_tokens + 1)
                .expect("cannot fit n_tokens into a usize")
        {
            return Err(BackendError::FailedTokenization);
        }

        let offset = self.llama_batch.n_tokens;
        let offset_usize = usize::try_from(offset).expect("cannot fit n_tokens into a usize");
        unsafe {
            // batch.token   [batch.n_tokens] = id;
            self.llama_batch.token.add(offset_usize).write(token);
            // batch.pos     [batch.n_tokens] = pos,
            self.llama_batch.pos.add(offset_usize).write(pos);
            // batch.n_seq_id[batch.n_tokens] = seq_ids.size();
            self.llama_batch.n_seq_id.add(offset_usize).write(
                hayride_llama_rs_sys::llama_seq_id::try_from(seq_ids.len())
                    .expect("cannot fit seq_ids.len() into a llama_seq_id"),
            );
            // for (size_t i = 0; i < seq_ids.size(); ++i) {
            //     batch.seq_id[batch.n_tokens][i] = seq_ids[i];
            // }
            for (i, seq_id) in seq_ids.iter().enumerate() {
                let tmp = *self.llama_batch.seq_id.add(offset_usize);
                tmp.add(i).write(*seq_id);
            }
            // batch.logits  [batch.n_tokens] = logits;
            self.llama_batch
                .logits
                .add(offset_usize)
                .write(i8::from(logits));
        }

        if logits {
            self.initialized_logits.push(offset);
        } else {
            self.initialized_logits.retain(|l| l != &offset);
        }

        self.llama_batch.n_tokens += 1;

        Ok(())
    }

    pub fn n_tokens(&self) -> i32 {
        self.llama_batch.n_tokens
    }

    pub fn batch(&self) -> hayride_llama_rs_sys::llama_batch {
        self.llama_batch
    }

    pub fn clear(&mut self) {
        self.llama_batch.n_tokens = 0;
        self.initialized_logits.clear();
    }
}

impl Drop for LlamaBatch {
    /// Drops the `LlamaBatch`.
    fn drop(&mut self) {
        if self.allocated > 0 {
            unsafe { hayride_llama_rs_sys::llama_batch_free(self.llama_batch) };
        }
    }
}

// write the output string to the writer blocking the thread
// Can be used to write output or errors to the stream
// Returns BackendError::FailedToWriteOutput on failure
fn write_output<W: tokio::io::AsyncWrite + Unpin>(
    mut writer: W,
    output: &str,
) -> Result<(), BackendError> {
    block_in_place(|| {
        let rt = Runtime::new().map_err(|_| BackendError::FailedToWriteOutput)?;
        rt.block_on(async {
            writer
                .write_all(output.as_bytes())
                .await
                .map_err(|_| BackendError::FailedToWriteOutput)
        })
    })
}
