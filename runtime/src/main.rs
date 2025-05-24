mod runtime;

use anyhow::Result;
use rama::http::Request;
use runtime::Runtime;

static PATH_TO_COMPONENT: &str = "../component/target/wasm32-wasip2/release/component.wasm";

#[tokio::main]
async fn main() -> Result<()> {
    let request = Request::builder()
        .method("GET")
        .uri("https://www.rust-lang.org/")
        .header("X-Custom-Foo", "Bar")
        .body("<H1>Hello !</H1>")?;

    let mut runtime = Runtime::new()?;
    let id = runtime.add_instance(PATH_TO_COMPONENT)?;

    let result = runtime.call_handle_request(id, "request".into())?;
    println!("Handle Request Function returned:\n\t-> \"{}\"", result);

    let result = runtime.call_handle_response(id, "response".into())?;
    println!("Handle Response Function returned:\n\t-> \"{}\"", result);

    Ok(())
}
