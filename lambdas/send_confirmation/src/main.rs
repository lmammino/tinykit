use aws_lambda_events::event::sqs::SqsEvent;
use aws_sdk_ses::types::{Body, Content, Destination, Message};
use jsonwebtoken::{encode, EncodingKey, Header};
use lambda_runtime::{run, service_fn, tracing, Error, LambdaEvent};
use shared::{SubscribeConfirmationTokenClaims, SubscribeEventPayload};
use std::env;
include!(concat!(env!("OUT_DIR"), "/sam_env.rs"));

struct Config {
    env: SamEnv,
    dynamodb_client: aws_sdk_dynamodb::Client,
    ses_client: aws_sdk_ses::Client,
    token_secret: EncodingKey,
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
                &config.token_secret,
            )?;
            tracing::info!("Confirmation token: {}", confirmation_token_token);

            let confirmation_url = format!(
                "{}?token={}",
                config.env.confirmation_endpoint, confirmation_token_token
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
                .source(&config.env.sender_email)
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
    let env = SamEnv::init_from_env().unwrap();

    let config = aws_config::load_from_env().await;
    let dynamodb_client = aws_sdk_dynamodb::Client::new(&config);
    let ses_client = aws_sdk_ses::Client::new(&config);
    let token_secret = EncodingKey::from_secret(env.token_secret.as_ref());

    let config = Config {
        env,
        dynamodb_client,
        ses_client,
        token_secret,
    };

    tracing::init_default_subscriber();

    run(service_fn(|event| function_handler(event, &config))).await
}
