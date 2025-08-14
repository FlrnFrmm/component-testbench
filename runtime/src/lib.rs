use std::cell::RefCell;
use std::collections::HashMap;
use std::path::Path;
use std::str::FromStr;

use anyhow::{Result, anyhow};
use rama::http::{HeaderName, HeaderValue, Request as RamaRequest, Uri};
use wasmtime::component::{Component, Linker, Resource, ResourceTable, TypedFunc, bindgen};
use wasmtime::{Engine, Store};
use wasmtime_wasi::p2::{IoView, WasiCtx, WasiCtxBuilder, WasiView};

pub type Request = ();
pub type Router = TypedFunc<(Resource<Request>,), (Result<(), String>,)>;

bindgen!({
    path: "../wit/",
    world: "crossroads",
    with: {
        "wit:crossroads/types/request": Request,
    }
});

pub(self) use wit::crossroads::types::{Host, HostRequest};

pub struct ComponentRunStates {
    pub wasi_ctx: WasiCtx,
    pub table: ResourceTable,
    pub requests: HashMap<u32, RamaRequest>,
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
    fn headers(&mut self, self_: Resource<Request>) -> Result<Vec<(String, String)>, String> {
        let request = self
            .requests
            .get(&self_.rep())
            .ok_or_else(|| "Request not in resource table".to_string())?;
        let header = request
            .headers()
            .iter()
            .map(|(key, value)| (key.to_string(), value.to_str().unwrap().to_string()))
            .collect();
        Ok(header)
    }

    fn set_header(
        &mut self,
        self_: Resource<Request>,
        key: String,
        value: String,
    ) -> Result<(), String> {
        let header_key = HeaderName::from_str(&key).map_err(|err| err.to_string())?;
        let header_value = HeaderValue::from_str(&value).map_err(|err| err.to_string())?;
        self.requests
            .get_mut(&self_.rep())
            .ok_or_else(|| "Request not in resource table".to_string())?
            .headers_mut()
            .insert(header_key, header_value);
        Ok(())
    }

    fn uri(&mut self, self_: Resource<Request>) -> Result<String, String> {
        let request = self
            .requests
            .get(&self_.rep())
            .ok_or_else(|| "Request not in resource table".to_string())?;
        Ok(request.uri().to_string())
    }

    fn set_uri(&mut self, self_: Resource<Request>, uri: String) -> Result<(), String> {
        let uri = Uri::from_str(&uri)
            .map_err(|err| format!("Error assigning uri: {}", err.to_string()))?;
        let request = self
            .requests
            .get_mut(&self_.rep())
            .ok_or_else(|| "Request not in resource table".to_string())?;
        *request.uri_mut() = uri;
        Ok(())
    }

    fn drop(&mut self, rep: Resource<Request>) -> wasmtime::Result<()> {
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
        wit::crossroads::types::add_to_linker(&mut linker, |state| state)?;
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

        let interface_namespace = "wit:crossroads/router@0.1.0";
        let interface_idx = instance
            .get_export_index(&mut self.store, None, interface_namespace)
            .expect(&format!("Cannot get `{}` interface", interface_namespace));

        let parent_export_idx = Some(&interface_idx);
        let func_id_handle_request = instance
            .get_export_index(&mut self.store, parent_export_idx, "handle")
            .expect(&format!("Cannot get `{}` function", "handle"));

        let func_handle_request = instance
            .get_func(&mut self.store, func_id_handle_request)
            .expect("Unreachable since we've got func_idx");

        let handle = func_handle_request
            .typed::<(Resource<Request>,), (Result<(), String>,)>(&self.store)?;

        self.instances.insert(id, handle);

        Ok(id)
    }

    pub fn call_handle(&mut self, id: usize, request: RamaRequest) -> Result<RamaRequest> {
        let resource = self.store.data_mut().table.push(())?;
        let resource_id = resource.rep();
        self.store.data_mut().requests.insert(resource_id, request);
        let Some(router) = self.instances.get(&id) else {
            anyhow::bail!("Couldn't find function with id {}", id);
        };
        let (result,) = router.call(&mut self.store, (resource,))?;
        result.map_err(|error_message| anyhow!("Component error: {}", error_message))?;
        router.post_return(&mut self.store)?;
        let Some(rama_request) = self.store.data_mut().requests.remove(&resource_id) else {
            anyhow::bail!("Couldn't find request ref cell with id {}", resource_id);
        };
        Ok(rama_request)
    }
}
