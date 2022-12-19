use std::io::{Cursor, Read, Seek};
use std::path::Path;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::RwLock;

use serde::{Deserialize, Serialize};
use wasi_common::file::FileCaps;
use wasi_common::pipe::{ReadPipe, WritePipe};
use wasmtime::{Engine, Linker, Module, Store};
use wasmtime_wasi::sync::WasiCtxBuilder;
use wasmtime_wasi::WasiCtx;

use crate::error::HostError;

/// Represents a WASM plugin
pub struct Plugin {
    pipe_in: Arc<RwLock<Cursor<Vec<u8>>>>,
    pipe_out: Arc<RwLock<Cursor<Vec<u8>>>>,
    store: Mutex<Store<WasiCtx>>,
    module: Module,
    linker: Linker<WasiCtx>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Message<T> {
    command: String,
    payload: T,
}

impl Plugin {
    pub fn from_path<P: AsRef<Path>>(engine: &Engine, path: P) -> Result<Plugin, HostError> {
        let module = Module::from_file(&engine, path).map_err(HostError::WasmModuleLoadFailed)?;
        Plugin::from_module(engine, module)
    }

    pub fn from_module(engine: &Engine, module: Module) -> Result<Plugin, HostError> {
        let mut wasi = WasiCtxBuilder::new().inherit_stdio().build();
        let pipe_in = Arc::new(RwLock::new(Cursor::new(Vec::new())));
        let pipe_out = Arc::new(RwLock::new(Cursor::new(Vec::new())));
        wasi.insert_file(
            4,
            Box::new(ReadPipe::from_shared(pipe_in.clone())),
            FileCaps::all(),
        );
        wasi.insert_file(
            5,
            Box::new(WritePipe::from_shared(pipe_out.clone())),
            FileCaps::all(),
        );
        let mut store = Store::new(&engine, wasi);
        let mut linker = Linker::new(&engine);
        wasmtime_wasi::add_to_linker(&mut linker, |s| s)
            .map_err(HostError::WasmModuleLinkingFailed)?;
        linker
            .module(&mut store, "plugin", &module)
            .map_err(HostError::WasmModuleLinkingFailed)?;
        Ok(Plugin {
            pipe_in,
            pipe_out,
            store: Mutex::new(store),
            module,
            linker,
        })
    }

    pub fn invoke<T: Serialize>(&self, command: &str, payload: T) -> Result<(), HostError> {
        let msg = Message {
            command: command.to_string(),
            payload,
        };
        {
            let mut pipe = self.pipe_in.write().unwrap();
            serde_json::to_writer(&mut *pipe, &msg).map_err(HostError::JsonSerializationFailed)?;
            pipe.rewind().unwrap();
        }
        let mut store = self.store.lock().unwrap();
        let symbol = self.linker.get(&mut *store, "plugin", "hello").unwrap();
        let func = symbol.into_func().unwrap();
        func.typed::<(), (), _>(&*store)
            .map_err(HostError::WasmInvokeFailed)?
            .call(&mut *store, ())
            .map_err(HostError::WasmInvokeFailed)?;

        {
            let mut buf = Vec::new();
            let mut pipe = self.pipe_out.write().unwrap();
            pipe.rewind().unwrap();
            pipe.read_to_end(&mut buf).unwrap();
            dbg!(String::from_utf8_lossy(&buf));
        }

        Ok(())
    }
}
