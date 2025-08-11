use bindings::exports::wit::rama::router::{Guest as Router, Request};

#[allow(warnings)]
mod bindings;

struct Component;

impl Router for Component {
    fn handle_request(request: Request) -> Result<(), String> {
        let mut s = String::new();
        for (key, _) in request.headers()? {
            s += &key;
        }
        request.set_header("x-handle-request", &s)
    }

    fn handle_response(request: Request) -> Result<(), String> {
        let mut s = String::new();
        for (_, value) in request.headers()? {
            s += &value;
        }
        request.set_header("x-handle-response", &s)
    }
}

bindings::export!(Component with_types_in bindings);
