use std::collections::HashMap;
use std::path::Path;

use anyhow::{Result, anyhow};
use wasmtime::component::{Component, Linker, ResourceTable, TypedFunc};
use wasmtime::{Engine, Store};
use wasmtime_wasi::p2::{IoView, WasiCtx, WasiCtxBuilder, WasiView};

pub struct ComponentRunStates {
    pub wasi_ctx: WasiCtx,
    pub resource_table: ResourceTable,
}

impl IoView for ComponentRunStates {
    fn table(&mut self) -> &mut ResourceTable {
        &mut self.resource_table
    }
}

impl WasiView for ComponentRunStates {
    fn ctx(&mut self) -> &mut WasiCtx {
        &mut self.wasi_ctx
    }
}

struct Router {
    handle_request: TypedFunc<(String,), (String,)>,
    handle_response: TypedFunc<(String,), (String,)>,
}

pub struct Runtime {
    engine: Engine,
    linker: Linker<ComponentRunStates>,
    store: Store<ComponentRunStates>,
    instances: HashMap<usize, Router>,
}

impl Runtime {
    pub fn new() -> Result<Self> {
        let engine = wasmtime::Engine::default();
        let mut linker: Linker<ComponentRunStates> = Linker::new(&engine);
        wasmtime_wasi::p2::add_to_linker_sync(&mut linker)?;
        let wasi = WasiCtxBuilder::new().inherit_stdio().inherit_args().build();
        let state = ComponentRunStates {
            wasi_ctx: wasi,
            resource_table: ResourceTable::new(),
        };
        let store = Store::new(&engine, state);
        let instances = HashMap::new();
        let runtime = Self {
            engine,
            linker,
            store,
            instances,
        };
        Ok(runtime)
    }

    pub fn add_instance(&mut self, path_to_component: impl AsRef<Path>) -> Result<usize> {
        let id = self.instances.keys().max().unwrap_or(&0) + 1;

        let component = Component::from_file(&self.engine, path_to_component)?;
        let instance = self.linker.instantiate(&mut self.store, &component)?;

        let interface_namespace = "rama:component/router@0.1.0";
        let interface_idx = instance
            .get_export_index(&mut self.store, None, interface_namespace)
            .expect(&format!("Cannot get `{}` interface", interface_namespace));

        let parent_export_idx = Some(&interface_idx);
        let func_id_handle_request = instance
            .get_export_index(&mut self.store, parent_export_idx, "handle-request")
            .expect(&format!("Cannot get `{}` function", "handle-request"));

        let func_handle_request = instance
            .get_func(&mut self.store, func_id_handle_request)
            .expect("Unreachable since we've got func_idx");

        let handle_request = func_handle_request.typed::<(String,), (String,)>(&self.store)?;

        let func_id_handle_response = instance
            .get_export_index(&mut self.store, parent_export_idx, "handle-response")
            .expect(&format!("Cannot get `{}` function", "handle-response"));

        let func_handle_response = instance
            .get_func(&mut self.store, func_id_handle_response)
            .expect("Unreachable since we've got func_idx");

        let handle_response = func_handle_response.typed::<(String,), (String,)>(&self.store)?;

        let router = Router {
            handle_request,
            handle_response,
        };

        self.instances.insert(id, router);

        Ok(id)
    }

    pub fn call_handle_request(&mut self, id: usize, param: String) -> Result<String> {
        let Some(router) = self.instances.get(&id) else {
            anyhow::bail!("Couldn't find function with id {}", id);
        };
        let (result,) = router.handle_request.call(&mut self.store, (param,))?;
        router.handle_request.post_return(&mut self.store)?;
        Ok(result)
    }

    pub fn call_handle_response(&mut self, id: usize, param: String) -> Result<String> {
        let Some(router) = self.instances.get(&id) else {
            anyhow::bail!("Couldn't find function with id {}", id);
        };
        let (result,) = router.handle_response.call(&mut self.store, (param,))?;
        router.handle_response.post_return(&mut self.store)?;
        Ok(result)
    }
}
