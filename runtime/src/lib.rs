mod router;

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;
use std::str::FromStr;

use anyhow::{anyhow, Result};
use rama::http::{HeaderName, HeaderValue, Request as RamaRequest};
use wasmtime::component::{bindgen, Component, Linker, Resource, ResourceTable};
use wasmtime::{Engine, Store};
use wasmtime_wasi::p2::{IoView, WasiCtx, WasiCtxBuilder, WasiView};

use router::Router;

pub type Request = ();

bindgen!({
    path:"../wit/world.wit",
    world:"service",
    with: {
        "wit:rama/types/request": Request,
    }
});

pub(self) use wit::rama::types::{Host, HostRequest};

pub struct ComponentRunStates {
    pub wasi_ctx: WasiCtx,
    pub table: ResourceTable,
    pub requests: HashMap<u32, RefCell<RamaRequest>>,
}

impl IoView for ComponentRunStates {
    fn table(&mut self) -> &mut ResourceTable {
        &mut self.table
    }
}

impl WasiView for ComponentRunStates {
    fn ctx(&mut self) -> &mut WasiCtx {
        &mut self.wasi_ctx
    }
}

impl Host for ComponentRunStates {}

impl HostRequest for ComponentRunStates {
    fn headers(
        &mut self,
        self_: wasmtime::component::Resource<Request>,
    ) -> Result<Vec<(String, String)>, String> {
        let request = self
            .requests
            .get(&self_.rep())
            .ok_or_else(|| "Request not in resource table".to_string())?;
        let header = request
            .borrow()
            .headers()
            .iter()
            .map(|(key, value)| (key.to_string(), value.to_str().unwrap().to_string()))
            .collect();
        Ok(header)
    }

    fn set_header(
        &mut self,
        self_: wasmtime::component::Resource<Request>,
        key: wasmtime::component::__internal::String,
        value: wasmtime::component::__internal::String,
    ) -> Result<(), String> {
        let header_key = HeaderName::from_str(&key).map_err(|err| err.to_string())?;
        let header_value = HeaderValue::from_str(&value).map_err(|err| err.to_string())?;
        self.requests
            .get_mut(&self_.rep())
            .ok_or_else(|| "Request not in resource table".to_string())?
            .borrow_mut()
            .headers_mut()
            .insert(header_key, header_value);
        Ok(())
    }

    fn drop(&mut self, rep: wasmtime::component::Resource<Request>) -> wasmtime::Result<()> {
        Ok(())
    }
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
        let mut linker = Linker::new(&engine);
        wasmtime_wasi::p2::add_to_linker_sync(&mut linker)?;
        wit::rama::types::add_to_linker(&mut linker, |state| state)?;
        let wasi = WasiCtxBuilder::new().inherit_stdio().inherit_args().build();
        let state = ComponentRunStates {
            wasi_ctx: wasi,
            table: ResourceTable::new(),
            requests: HashMap::new(),
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

        let interface_namespace = "wit:rama/router@0.1.0";
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

        let handle_request = func_handle_request
            .typed::<(Resource<Request>,), (Result<(), String>,)>(&self.store)?;

        let func_id_handle_response = instance
            .get_export_index(&mut self.store, parent_export_idx, "handle-response")
            .expect(&format!("Cannot get `{}` function", "handle-response"));

        let func_handle_response = instance
            .get_func(&mut self.store, func_id_handle_response)
            .expect("Unreachable since we've got func_idx");

        let handle_response = func_handle_response
            .typed::<(Resource<Request>,), (Result<(), String>,)>(&self.store)?;

        let router = Router {
            handle_request,
            handle_response,
        };

        self.instances.insert(id, router);

        Ok(id)
    }

    pub fn call_handle_request(&mut self, id: usize, request: RamaRequest) -> Result<RamaRequest> {
        let resource = self.store.data_mut().table.push(())?;
        let resource_id = resource.rep();
        self.store
            .data_mut()
            .requests
            .insert(resource_id, RefCell::new(request));
        let Some(router) = self.instances.get(&id) else {
            anyhow::bail!("Couldn't find function with id {}", id);
        };
        let (result,) = router.handle_request.call(&mut self.store, (resource,))?;
        result.map_err(|error_message| anyhow!("Component error: {}", error_message))?;
        router.handle_request.post_return(&mut self.store)?;
        let Some(rama_request) = self.store.data_mut().requests.remove(&resource_id) else {
            anyhow::bail!("Couldn't find request ref cell with id {}", resource_id);
        };
        Ok(rama_request.into_inner())
    }

    pub fn call_handle_response(&mut self, id: usize, request: RamaRequest) -> Result<RamaRequest> {
        let resource = self.store.data_mut().table.push(())?;
        let resource_id = resource.rep();
        self.store
            .data_mut()
            .requests
            .insert(resource_id, RefCell::new(request));
        let Some(router) = self.instances.get(&id) else {
            anyhow::bail!("Couldn't find function with id {}", id);
        };
        let (result,) = router.handle_response.call(&mut self.store, (resource,))?;
        result.map_err(|error_message| anyhow!("Component error: {}", error_message))?;
        router.handle_response.post_return(&mut self.store)?;
        let Some(rama_request) = self.store.data_mut().requests.remove(&resource_id) else {
            anyhow::bail!("Couldn't find request ref cell with id {}", resource_id);
        };
        Ok(rama_request.into_inner())
    }
}
