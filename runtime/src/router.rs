use wasmtime::component::{Resource, TypedFunc};

pub struct Router {
    pub handle_request: TypedFunc<(Resource<()>,), (Result<(), String>,)>,
    pub handle_response: TypedFunc<(Resource<()>,), (Result<(), String>,)>,
}
