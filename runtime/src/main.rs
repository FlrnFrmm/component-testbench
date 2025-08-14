use anyhow::Result;
use rama::http::{Body, Request};

use runtime::Runtime;
use tokio::time::{Duration, Instant};

static PATH_TO_COMPONENT: &str = "../component/target/wasm32-wasip2/release/component.wasm";

#[tokio::main]
async fn main() -> Result<()> {
    let body = Body::new::<String>("<H1>Hello !</H1>".into());
    let request = Request::builder()
        .method("GET")
        .uri("https://faa4abb5-c37a-4ba1-89df-a12075997594.functions.runs.onstackit.cloud/")
        .header("X-Custom-Foo", "Bar")
        .body(body)?;

    let mut runtime = Runtime::new()?;
    let id = runtime.add_instance(PATH_TO_COMPONENT)?;

    let modified_request = runtime.call_handle(id, request)?;

    let runs = 1000;

    let mut duration_handle = Duration::ZERO;

    for _ in 0..runs {
        let body = Body::new::<String>("<H1>Hello !</H1>".into());
        let request = Request::builder()
            .method("GET")
            .uri("https://330752d1-487b-4c56-87a9-b4d3bfe946f1.functions.runs.onstackit.cloud")
            .header("X-Custom-Foo", "Bar")
            .body(body)?;

        let start = Instant::now();
        let modified_request = runtime.call_handle(id, request)?;
        let elapsed = start.elapsed();
        duration_handle += elapsed;
    }

    println!(
        "Total duration request: {} seconds",
        duration_handle.as_secs_f64()
    );
    println!(
        "Average duration request: {} seconds",
        duration_handle.as_secs_f64() / runs as f64
    );

    Ok(())
}
