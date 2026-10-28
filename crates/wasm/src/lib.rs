use std::path::{Path, PathBuf};

use async_trait::async_trait;
use fatima_engine::{
    handle_api_request_with_host, ApiRequest, CommandOutput, CommandRequest, FatimaError,
    FatimaHost, HttpRequest, Result, Secrets,
};
use js_sys::{Function, Promise, Reflect};
use serde_json::Value;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

#[wasm_bindgen]
pub async fn handle_api_request_json(input: String, host: JsValue) -> String {
    let response = match serde_json::from_str::<ApiRequest>(&input) {
        Ok(request) => handle_api_request_with_host(request, &JsHost::new(host)).await,
        Err(error) => fatima_engine::ApiResponse::error(error),
    };
    serde_json::to_string(&response).unwrap_or_else(|error| {
        format!(r#"{{"ok":false,"error":"failed to serialize response: {error}"}}"#)
    })
}

struct JsHost {
    host: JsValue,
}

impl JsHost {
    fn new(host: JsValue) -> Self {
        Self { host }
    }

    async fn call(&self, name: &str, args: &[JsValue]) -> Result<JsValue> {
        let function = Reflect::get(&self.host, &JsValue::from_str(name))
            .map_err(js_error)?
            .dyn_into::<Function>()
            .map_err(|_| FatimaError::message(format!("host.{name} is not a function")))?;
        let args_array = js_sys::Array::new();
        for arg in args {
            args_array.push(arg);
        }
        let value = function.apply(&self.host, &args_array).map_err(js_error)?;
        JsFuture::from(Promise::resolve(&value))
            .await
            .map_err(js_error)
    }

    async fn call_string(&self, name: &str, args: &[JsValue]) -> Result<String> {
        self.call(name, args)
            .await?
            .as_string()
            .ok_or_else(|| FatimaError::message(format!("host.{name} must return a string")))
    }

    async fn call_bool(&self, name: &str, args: &[JsValue]) -> Result<bool> {
        self.call(name, args)
            .await?
            .as_bool()
            .ok_or_else(|| FatimaError::message(format!("host.{name} must return a boolean")))
    }
}

#[async_trait(?Send)]
impl FatimaHost for JsHost {
    async fn env_vars(&self) -> Result<Secrets> {
        serde_wasm_bindgen::from_value(self.call("envVars", &[]).await?)
            .map_err(|error| FatimaError::message(error.to_string()))
    }

    async fn env_var(&self, key: &str) -> Result<Option<String>> {
        let value = self.call("envVar", &[JsValue::from_str(key)]).await?;
        if value.is_null() || value.is_undefined() {
            Ok(None)
        } else {
            Ok(value.as_string())
        }
    }

    async fn current_dir(&self) -> Result<PathBuf> {
        Ok(PathBuf::from(self.call_string("currentDir", &[]).await?))
    }

    async fn exists(&self, path: &Path) -> Result<bool> {
        self.call_bool("exists", &[path_arg(path)]).await
    }

    async fn read_to_string(&self, path: &Path) -> Result<String> {
        self.call_string("readText", &[path_arg(path)]).await
    }

    async fn write_string(&self, path: &Path, content: &str) -> Result<()> {
        self.call("writeText", &[path_arg(path), JsValue::from_str(content)])
            .await?;
        Ok(())
    }

    async fn create_dir_all(&self, path: &Path) -> Result<()> {
        self.call("createDirAll", &[path_arg(path)]).await?;
        Ok(())
    }

    async fn remove_file(&self, path: &Path) -> Result<()> {
        self.call("removeFile", &[path_arg(path)]).await?;
        Ok(())
    }

    async fn process_id(&self) -> Result<u32> {
        Ok(self.call("processId", &[]).await?.as_f64().unwrap_or(0.0) as u32)
    }

    async fn run_command(&self, request: CommandRequest) -> Result<CommandOutput> {
        serde_wasm_bindgen::from_value(
            self.call(
                "runCommand",
                &[serde_wasm_bindgen::to_value(&serde_json::json!({
                    "command": request.command,
                    "args": request.args,
                    "cwd": request.cwd.display().to_string(),
                    "env": request.env,
                }))
                .map_err(|error| FatimaError::message(error.to_string()))?],
            )
            .await?,
        )
        .map_err(|error| FatimaError::message(error.to_string()))
    }

    async fn http_json(&self, request: HttpRequest) -> Result<Value> {
        let value = self
            .call(
                "httpJson",
                &[serde_wasm_bindgen::to_value(&serde_json::json!({
                    "method": request.method,
                    "url": request.url,
                    "headers": request.headers,
                    "query": request.query,
                    "body": request.body,
                    "bearer": request.bearer,
                }))
                .map_err(|error| FatimaError::message(error.to_string()))?],
            )
            .await?;
        serde_wasm_bindgen::from_value(value)
            .map_err(|error| FatimaError::message(error.to_string()))
    }
}

fn path_arg(path: &Path) -> JsValue {
    JsValue::from_str(&path.display().to_string())
}

fn js_error(error: JsValue) -> FatimaError {
    FatimaError::message(
        error
            .as_string()
            .unwrap_or_else(|| "JavaScript host error".to_string()),
    )
}
