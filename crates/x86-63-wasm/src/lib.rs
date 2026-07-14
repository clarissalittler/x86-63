use wasm_bindgen::prelude::*;
use x86_63_core::{Command, Session, SourceModule};

#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
}

#[wasm_bindgen]
pub fn lessons_json() -> Result<String, JsValue> {
    serde_json::to_string(&x86_63_course::lesson_views()).map_err(js_error)
}

#[wasm_bindgen]
pub struct WasmSession {
    inner: Session,
}

#[wasm_bindgen]
impl WasmSession {
    #[wasm_bindgen(constructor)]
    pub fn new(modules_json: &str) -> Result<WasmSession, JsValue> {
        let modules: Vec<SourceModule> = serde_json::from_str(modules_json).map_err(js_error)?;
        let inner = Session::from_modules(modules).map_err(|error| {
            JsValue::from_str(&serde_json::to_string(&error).unwrap_or_else(|_| error.to_string()))
        })?;
        Ok(Self { inner })
    }

    pub fn execute(&mut self, command_json: &str) -> Result<String, JsValue> {
        let command: Command = serde_json::from_str(command_json).map_err(js_error)?;
        serde_json::to_string(&self.inner.execute(command)).map_err(js_error)
    }

    pub fn view_json(&self) -> Result<String, JsValue> {
        serde_json::to_string(&self.inner.view()).map_err(js_error)
    }

    pub fn program_json(&self) -> Result<String, JsValue> {
        serde_json::to_string(&self.inner.program()).map_err(js_error)
    }
}

fn js_error(error: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&error.to_string())
}
