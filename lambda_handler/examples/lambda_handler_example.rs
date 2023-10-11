/// Examples

use lambda_runtime::{Error as LambdaError, LambdaEvent};
use serde::Serialize;

use lambda_handler::events::{S3Event, SnsEvent, SqsEvent};
use lambda_handler::handler::LambdaHandler;


#[derive(Debug, Serialize)]
pub struct Response {
    pub req_id: String,
    pub body: String,
}


async fn handle_s3_event(event: LambdaEvent<S3Event>) -> Result<Response, LambdaError> {
    Ok(Response {
        req_id: event.context.request_id,
        body: "I handled an S3 event!".to_string(),
    })
}

async fn handle_sns_event(event: LambdaEvent<SnsEvent>) -> Result<Response, LambdaError> {
    Ok(Response {
        req_id: event.context.request_id,
        body: "I handled an SNS event!".to_string(),
    })
}

async fn handle_sqs_event(event: LambdaEvent<SqsEvent>) -> Result<Response, LambdaError> {
    Ok(Response {
        req_id: event.context.request_id,
        body: "I handled an SQS event!".to_string(),
    })
}

#[tokio::main]
async fn main() -> Result<(), LambdaError> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .without_time()
        .init();

    let handler = LambdaHandler::<Response>::new()
        .route("ObjectCreated:Put", handle_s3_event)
        .route("ObjectCreated:Delete", handle_s3_event)
        .route("arn:aws:sns:us-east-1:246796806071:snsNetTest", handle_sns_event)
        .route("arn:aws:sqs:us-west-2:123456789012:SQSQueue", handle_sqs_event);

    lambda_runtime::run(handler).await
}
