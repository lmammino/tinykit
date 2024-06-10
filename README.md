# tinykit

## Create test campaign in DynamoDB from CLI:

```bash
aws dynamodb put-item --table-name tinykit-tinykitdev-campaigns --item '{"campaign_id": {"S": "test"},"name": {"S": "test campaign"},"reward_s3_key": {"S": ""},"email_template_s3_key": {"S": ""},"thank_you_message": {"S": "Thanks for joining TEST campaign!"}}'
```