# tinykit

# Setup the project

## 0. Deploy the stack

Populate all the necessary stack parameters (including your test email
`SenderEmail`)

then run:

```bash
sam validate --lint && sam build --beta-features && sam deploy
```

## 1. Create a test campaign in DynamoDB from CLI:

```bash
aws dynamodb put-item --table-name tinykit-tinykitdev-campaigns --item '{"campaign_id": {"S": "test"},"name": {"S": "test campaign"},"reward_s3_key": {"S": ""},"email_template_s3_key": {"S": ""},"thank_you_message": {"S": "Thanks for joining TEST campaign!"}}'
```

## 2. Validate the test email

Go in the AWS Console, open the SES service, and validate the email address you
used as `SenderEmail` in the stack parameters.

## 3. Register a test user in the newsletter

Go to your API Gateway endpoint:

```bash
https://<apiGatewayURL>/form/test
```
