use lambda_http::{
    request::RequestContext, run, service_fn, tracing, Body, Error, Request, RequestExt,
    RequestPayloadExt, Response,
};

fn create_form(target_url: &str) -> String {
    format!(
        r#"
    <html>
        <head>
            <title>Subscribe to our newsletter</title>
        </head>
        <body>
            <form action="{target_url}" method="post">
                <label for="email">Email:</label>
                <input required type="email" id="email" name="email">
                <input type="submit" value="Submit">
            </form>
        </body>
    </html>
    "#
    )
}

async fn function_handler(event: Request) -> Result<Response<Body>, Error> {
    let request_context = event.request_context();
    let form_submit_url = match request_context {
        RequestContext::ApiGatewayV2(api_gateway_v2) => {
            let domain_name = api_gateway_v2.domain_name.expect("domain_name not found");
            let path = api_gateway_v2.http.path.expect("path not found");
            format!("https://{}{}", domain_name, path)
        }
        _ => {
            return Err("Unsupported request context".into());
        }
    };
    let form_html = create_form(&form_submit_url);

    // Return something that implements IntoResponse.
    // It will be serialized to the right response event automatically by the runtime
    let resp = Response::builder()
        .status(200)
        .header("content-type", "text/html")
        .body(form_html.into())
        .map_err(Box::new)?;
    Ok(resp)
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing::init_default_subscriber();

    run(service_fn(function_handler)).await
}
