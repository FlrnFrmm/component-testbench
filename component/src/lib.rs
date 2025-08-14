use bindings::exports::wit::crossroads::router::{Guest as Router, Request};

use url::Url;
use uuid::Uuid;
#[allow(warnings)]
mod bindings;

struct Component;

impl Router for Component {
    fn handle(request: Request) -> Result<(), String> {
        let uri = request.uri()?;
        let mut url = Url::parse(uri.as_str()).map_err(|e| format!("Invalid URL: {}", e))?;
        let host = url.host_str().ok_or("Uri has no host found")?;
        let mut host_iter = host.split('.');
        let Some(uuid) = host_iter.next() else {
            return Err("Invalid host, no subdomain".to_string());
        };
        Uuid::parse_str(uuid).map_err(|e| format!("Invalid subdomainUUID: {}", e))?;
        request.set_header("UUID", uuid)?;
        let new_host = host_iter.collect::<Vec<&str>>().join(".");
        request.set_header("HOST", &new_host)?;
        url.set_host(Some(&new_host))
            .map_err(|e| format!("Invalid new host: {}", e))?;
        request.set_uri(url.as_str())
    }
}

bindings::export!(Component with_types_in bindings);
