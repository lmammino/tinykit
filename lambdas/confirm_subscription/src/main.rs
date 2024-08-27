use aws_sdk_dynamodb::types::AttributeValue;
use aws_sdk_s3::presigning::PresigningConfig;
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use lambda_http::{
    http::StatusCode, run, service_fn, tracing, Body, Error, Request, RequestExt, Response,
};
use shared::SubscribeConfirmationTokenClaims;
use std::{env, time::Duration};

include!(concat!(env!("OUT_DIR"), "/sam_env.rs"));

struct Config {
    dynamodb_client: aws_sdk_dynamodb::Client,
    s3_client: aws_sdk_s3::Client,
    campaigns_table: String,
    subscriptions_table: String,
    decoding_key: DecodingKey,
    resources_bucket: String,
}

async fn function_handler(event: Request, config: &Config) -> Result<Response<Body>, Error> {
    //  1. validate subscription_id
    //     (and get campaign_id, reward_s3_key, ...)
    //     (if already sent, stop)

    //  3. update subscription record (confirmed)
    //  4. send eventbridgde event
    //      subscription_confirmation_confirmed
    //  5. respond with a redirect to the pre_signed_url

    let token = event
        .query_string_parameters_ref()
        .and_then(|params| params.first("token"))
        .and_then(|token| {
            decode::<SubscribeConfirmationTokenClaims>(
                token,
                &config.decoding_key,
                &Validation::new(Algorithm::HS256),
            )
            .ok()
        });

    let token_data = match token {
        Some(token_data) => token_data,
        None => {
            return Ok(Response::builder()
                .status(400)
                .body("Invalid token".into())
                .map_err(Box::new)?)
        }
    };

    // get campaign details from DynamoDB
    let get_campaign_result = config
        .dynamodb_client
        .get_item()
        .table_name(config.campaigns_table.clone())
        .key(
            "campaign_id",
            AttributeValue::S(token_data.claims.campaign_id.clone()),
        )
        .send()
        .await
        .map_err(Box::new)?;

    if get_campaign_result.item.is_none() {
        return Ok(Response::builder()
            .status(400)
            .body("Invalid campaign".into())
            .map_err(Box::new)?);
    }

    // TODO: use https://docs.rs/serde_dynamo/latest/serde_dynamo/ to deserialize the item in a better/cooler way
    let campaign_item = get_campaign_result.item.unwrap();
    let reward_s3_key = campaign_item
        .get("reward_s3_key")
        .unwrap()
        .as_s()
        .unwrap()
        .as_str();

    // Create pre-signed URL to get the reward file
    let expires_in = Duration::from_secs(60);
    let presigned_request = config
        .s3_client
        .get_object()
        .bucket(&config.resources_bucket)
        .key(reward_s3_key)
        .presigned(PresigningConfig::expires_in(expires_in)?)
        .await?;

    let redirect_uri = presigned_request.uri();

    let resp = Response::builder()
        .status(StatusCode::FOUND)
        .header("location", redirect_uri)
        .body(Body::Empty)
        .map_err(Box::new)?;
    Ok(resp)
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let sam_env = SamEnv::init_from_env().unwrap();

    // TODO: review if there's a better way to do this (maybe `clap` or `envconfig`)
    let campaigns_table = sam_env.campaigns_table;
    let subscriptions_table = sam_env.subscriptions_table;
    let token_secret = sam_env.token_secret;
    let resources_bucket = sam_env.resources_bucket;

    let config = aws_config::load_from_env().await;
    let dynamodb_client = aws_sdk_dynamodb::Client::new(&config);
    let s3_client = aws_sdk_s3::Client::new(&config);

    let decoding_key = DecodingKey::from_secret(token_secret.as_ref());

    let config = Config {
        dynamodb_client,
        s3_client,
        campaigns_table,
        subscriptions_table,
        decoding_key,
        resources_bucket,
    };

    tracing::init_default_subscriber();

    run(service_fn(|event| function_handler(event, &config))).await
}
