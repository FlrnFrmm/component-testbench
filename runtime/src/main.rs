use anyhow::Result;
use rama::http::{Body, Request};

use runtime::Runtime;

static PATH_TO_COMPONENT: &str = "../component/target/wasm32-wasip2/release/component.wasm";

#[tokio::main]
async fn main() -> Result<()> {
    let body = Body::new::<String>("<H1>Hello !</H1>".into());
    let request = Request::builder()
        .method("GET")
        .uri("https://www.rust-lang.org/")
        .header("X-Custom-Foo", "Bar")
        .body(body)?;

    let mut runtime = Runtime::new()?;
    let id = runtime.add_instance(PATH_TO_COMPONENT)?;

    let modified_request = runtime.call_handle_request(id, request)?;
    println!(
        "Handle Request Function returned:\n\t-> {:?}",
        modified_request
    );

    let modified_request = runtime.call_handle_response(id, modified_request)?;
    println!(
        "Handle Response Function returned:\n\t-> {:?}",
        modified_request
    );

    Ok(())
}
