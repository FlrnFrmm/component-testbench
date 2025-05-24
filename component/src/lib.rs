use bindings::exports::rama::component::router::Guest as Router;

#[allow(warnings)]
mod bindings;

struct Component;

impl Router for Component {
    fn handle_request(input: String) -> String {
        input.chars().rev().collect()
    }

    fn handle_response(input: String) -> String {
        input.chars().rev().collect()
    }
}

bindings::export!(Component with_types_in bindings);
