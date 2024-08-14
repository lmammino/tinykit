use aws_lambda_events::event::sqs::SqsEvent;
use aws_sdk_ses::types::{Body, Content, Destination, Message};
use jsonwebtoken::{encode, EncodingKey, Header};
use lambda_runtime::{run, service_fn, tracing, Error, LambdaEvent};
use shared::{SubscribeConfirmationTokenClaims, SubscribeEventPayload};
use std::env;

struct Config {
    dynamodb_client: aws_sdk_dynamodb::Client,
    ses_client: aws_sdk_ses::Client,
    campaigns_table: String,
    subscriptions_table: String,
    sender_email: String,
    token_secret: String,
    confirmation_endpoint: String,
}

async fn function_handler(event: LambdaEvent<SqsEvent>, config: &Config) -> Result<(), Error> {
    // TODO: validate campaign id and subscription id
    // TODO: get the template from database
    // TODO: tracking pixel
    // TODO: implement list of failed messages

    // Extract some useful information from the request
    for record in event.payload.records {
        if let Some(sqs_body) = record.body {
            // generate unique token (needs to have campaign id and subscription id)
            let sqs_message: SubscribeEventPayload = serde_json::from_str(&sqs_body)?;
            tracing::info!("Received message: {:?}", sqs_message);

            let confirmation_token_claims = SubscribeConfirmationTokenClaims::new(
                sqs_message.subscription_id.clone(),
                sqs_message.campaign_id.clone(),
                sqs_message.email.clone(),
                60 * 60 * 24,
            );
            let confirmation_token_token = encode(
                &Header::default(),
                &confirmation_token_claims,
                &EncodingKey::from_secret(config.token_secret.as_ref()),
            )?;
            tracing::info!("Confirmation token: {}", confirmation_token_token);

            let confirmation_url = format!(
                "{}?token={}",
                config.confirmation_endpoint, confirmation_token_token
            );
            tracing::info!("Confirmation url: {}", confirmation_url);

            // Generate the email content
            let subject = Content::builder()
                .data("Please confirm your subscription".to_string())
                .build()?;

            let message_text = Content::builder()
                .data(format!(
                    "Click here to confirm your subscription: {}",
                    confirmation_url
                ))
                .build()?;

            let message_html = Content::builder()
                .data(format!(
                    "Click <a href=\"{}\">here</a> to confirm your subscription",
                    confirmation_url
                ))
                .build()?;

            let email_body = Body::builder()
                .text(message_text)
                .html(message_html)
                .build();

            let send_result = config
                .ses_client
                .send_email()
                .source(&config.sender_email)
                .destination(
                    Destination::builder()
                        .to_addresses(&sqs_message.email)
                        .build(),
                )
                .message(Message::builder().subject(subject).body(email_body).build())
                .send()
                .await?;

            tracing::info!("Email sent: {:?}", send_result);

            // TODO: update the send at timestamp in the subscription record
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let campaigns_table = env::var("CAMPAIGNS_TABLE").expect("CAMPAIGNS_TABLE missing");
    let subscriptions_table = env::var("SUBSCRIPTIONS_TABLE").expect("SUBSCRIPTIONS_TABLE missing");
    let sender_email = env::var("SENDER_EMAIL").expect("SENDER_EMAIL missing");
    let token_secret = env::var("TOKEN_SECRET").expect("TOKEN_SECRET missing");
    let confirmation_endpoint = env::var("CONFIRMATION_ENDPOINT").expect("API_GATEWAY_URL missing");

    let config = aws_config::load_from_env().await;
    let dynamodb_client = aws_sdk_dynamodb::Client::new(&config);
    let ses_client = aws_sdk_ses::Client::new(&config);

    let config = Config {
        dynamodb_client,
        ses_client,
        campaigns_table,
        subscriptions_table,
        sender_email,
        token_secret,
        confirmation_endpoint,
    };

    tracing::init_default_subscriber();

    run(service_fn(|event| function_handler(event, &config))).await
}
