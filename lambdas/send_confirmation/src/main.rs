use std::env;

use aws_lambda_events::event::sqs::SqsEvent;
use aws_sdk_ses::types::{Body, Content, Destination, Message};
use lambda_runtime::{run, service_fn, tracing, Error, LambdaEvent};
use shared::SubscribeEventPayload;

struct Config {
    dynamodb_client: aws_sdk_dynamodb::Client,
    ses_client: aws_sdk_ses::Client,
    campaigns_table: String,
    subscriptions_table: String,
    sender_email: String,
}

async fn function_handler(event: LambdaEvent<SqsEvent>, config: &Config) -> Result<(), Error> {
    // TODO: implement list of failed messages

    // Extract some useful information from the request
    for record in event.payload.records {
        // TODO: generate unique token
        // TODO: store token in the database
        // TODO: generate message content with link
        // TODO: generate subject

        let subject = Content::builder()
            .data("Please confirm your subscription".to_string())
            .build()?;

        let message_text = Content::builder()
            .data("Click here to confirm your subscription".to_string())
            .build()?;

        let email_body = Body::builder().text(message_text).build();

        println!("{:?}", record.body);
        
        if let Some(sqs_body) = record.body {
            let message: SubscribeEventPayload = serde_json::from_str(&sqs_body)?;
            config
                .ses_client
                .send_email()
                .source(&config.sender_email)
                .destination(Destination::builder().to_addresses(&message.email).build())
                .message(Message::builder().subject(subject).body(email_body).build())
                .send()
                .await?;
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let campaigns_table = env::var("CAMPAIGNS_TABLE").expect("CAMPAIGNS_TABLE missing");
    let subscriptions_table = env::var("SUBSCRIPTIONS_TABLE").expect("SUBSCRIPTIONS_TABLE missing");
    let sender_email = env::var("SENDER_EMAIL").expect("SENDER_EMAIL missing");

    let config = aws_config::load_from_env().await;
    let dynamodb_client = aws_sdk_dynamodb::Client::new(&config);
    let ses_client = aws_sdk_ses::Client::new(&config);

    let config = Config {
        dynamodb_client,
        ses_client,
        campaigns_table,
        subscriptions_table,
        sender_email,
    };

    tracing::init_default_subscriber();

    run(service_fn(|event| function_handler(event, &config))).await
}
