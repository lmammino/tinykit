use aws_sdk_dynamodb::types::AttributeValue;
use lambda_http::request::RequestContext;
use lambda_http::{
    run, service_fn, tracing, Body, Error, Request, RequestExt, RequestPayloadExt, Response,
};
use serde::Deserialize;
use serde_json::json;
use shared::SubscribeEventPayload;
use std::env;
use validators::models::Host;
use validators::prelude::*;

#[derive(Validator)]
#[validator(email(
    comment(Disallow),
    ip(Allow),
    local(Allow),
    at_least_two_labels(Allow),
    non_ascii(Allow)
))]
pub struct Email {
    pub local_part: String,
    pub need_quoted: bool,
    pub domain_part: Host,
}

#[derive(Debug, Deserialize)]
struct FormPayload {
    email: String,
}

#[derive(Debug)]
struct Config {
    dynamodb_client: aws_sdk_dynamodb::Client,
    sqs_client: aws_sdk_sqs::Client,
    // TODO: SQS client
    campaigns_table: String,
    subscriptions_table: String,
    email_queue: String,
}

async fn function_handler(event: Request, config: &Config) -> Result<Response<Body>, Error> {
    /*
       1. validate email
       2. validate campaign_id
       3. create subscription record
       4. put send_confirmation_email job in the queue
       (OPTIONAL) 5. send eventbridge event subscription_started
       6. return success message
    */
    let path_parameters = &event.path_parameters();
    let request_context = &event.request_context();

    let campaign_id = path_parameters
        .first("campaign_id")
        .expect("Campaign ID missing");

    let payload: FormPayload = event
        .payload()
        // TODO: properly handle these errors and return a response
        .expect("Invalid payload, could not deserialize.")
        .expect("Payload missing");

    // 1. validate email
    if Email::parse_str(&payload.email).is_err() {
        return Ok(Response::builder()
            .status(400)
            .header("content-type", "text/html")
            .body("Invalid email".into())
            .map_err(Box::new)?);
    }

    // 2. validate campaign_id
    let campaign = config
        .dynamodb_client
        .get_item()
        .table_name(&config.campaigns_table)
        .key("campaign_id", AttributeValue::S(campaign_id.to_string()))
        .send()
        .await
        .expect("Failed to get campaign");

    if campaign.item.is_none() {
        return Ok(Response::builder()
            .status(404)
            .header("content-type", "text/html")
            .body("Campaign not found".into())
            .map_err(Box::new)?);
    }

    // 3. save subscription record
    let ip: String = match request_context {
        RequestContext::ApiGatewayV2(context) => context.http.source_ip.clone().unwrap_or_default(),
        _ => {
            return Err("Unsupported request context".into());
        }
    };

    let subscription_id = cuid::cuid2();
    // subscription_id | campaign_id | IP | fingerprint | email | sent_at | opened_at | confirmed_at | unsubscribed_at
    config
        .dynamodb_client
        .put_item()
        .table_name(&config.subscriptions_table)
        .item(
            "subscription_id",
            AttributeValue::S(subscription_id.to_string()),
        )
        .item("campaign_id", AttributeValue::S(campaign_id.to_string()))
        .item("email", AttributeValue::S(payload.email.to_string()))
        .item("ip", AttributeValue::S(ip))
        .send()
        .await
        .expect("Failed to save subscription");

    // 4. put send_confirmation_email job in the queue
    let sqs_message_body = SubscribeEventPayload {
        subscription_id,
        campaign_id: campaign_id.to_string(),
        email: payload.email.clone(),
    };
    let result = config
        .sqs_client
        .send_message()
        .queue_url(&config.email_queue)
        .message_body(serde_json::to_string(&sqs_message_body).unwrap())
        .send()
        .await;

    // TODO: if this fails, we need to delete the subscription record and return a 502 error
    let sqs_message_id = result.unwrap().message_id.unwrap();
    tracing::info!(id = sqs_message_id, "Inserted message in the queue");

    let message = format!("Hello {campaign_id}, {}", payload.email);

    // TODO: start from here --- put message in the Queue

    // Return something that implements IntoResponse.
    // It will be serialized to the right response event automatically by the runtime
    let resp = Response::builder()
        .status(200)
        .header("content-type", "text/html")
        .body(message.into())
        .map_err(Box::new)?;
    Ok(resp)
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let campaigns_table = env::var("CAMPAIGNS_TABLE").expect("CAMPAIGNS_TABLE missing");
    let subscriptions_table = env::var("SUBSCRIPTIONS_TABLE").expect("SUBSCRIPTIONS_TABLE missing");
    let email_queue = env::var("EMAIL_QUEUE").expect("EMAIL_QUEUE missing");

    let config = aws_config::load_from_env().await;
    let dynamodb_client = aws_sdk_dynamodb::Client::new(&config);
    let sqs_client = aws_sdk_sqs::Client::new(&config);

    let config = Config {
        dynamodb_client,
        sqs_client,
        campaigns_table,
        subscriptions_table,
        email_queue,
    };

    tracing::init_default_subscriber();

    run(service_fn(|event| function_handler(event, &config))).await
}
