# Lambda Router

`lambda-router` is a Rust library for creating AWS Lambda functions which handle
multiple requests, routing each based on the type of the request and its name (for
instance, an S3 event name, an SNS topic name, etc.).

It makes for extremely simple code:

```rust
use lambda_runtime::{Error as LambdaError, LambdaEvent};
use serde::Serialize;

use lambda_handler::events::{S3Event, SnsEvent, SqsEvent};
use lambda_handler::LambdaHandler;


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

    let router = LambdaHandler::<Response>::new()
        .route("ObjectCreated:Put", handle_s3_event)
        .route("ObjectCreated:Delete", handle_s3_event)
        .route("arn:aws:sns:us-east-1:246796806071:snsNetTest", handle_sns_event)
        .route("arn:aws:sqs:us-west-2:123456789012:SQSQueue", handle_sqs_event);

    lambda_runtime::run(router).await
}
```

## Explanation

The code begins with our imports; we need the lambda_runtime for running, and serde
for serializing our response data. From `lambda _route::events` we import three event
types; these are re-exported from the `aws_lambda_events` create, but we have to use
them here because we add a trait for internal use.

The `Response` struct is just what we have decided on here; there are three requirements
here:

- It must implement `Debug` and `Serialize` (which are easily derived, as here)
- It must be used in the return type of _all_ handling functions
- It is used to create a `LambdaHandler` instance which expects handling functions to
  return it, with `LambdaHandler::<Response>::new()`

Next we define three async functions that handle the events themselves; this is where
most of the code in your binary actually resides. Each takes an `event` of type
`LambdaEvent<T>`, where `T` is the type of event it handles. And it returns a
`Result<Response, LambdaError>`.

Finally, we have our async `main` function. We set up tracing, and then create our
router:

```rust
    let router = LambdaHandler::<Response>::new()
        .route("ObjectCreated:Put", handle_s3_event)
        .route("ObjectCreated:Delete", handle_s3_event)
        .route("arn:aws:sns:us-east-1:246796806071:snsNetTest", handle_sns_event)
        .route("arn:aws:sqs:us-west-2:123456789012:SQSQueue", handle_sqs_event);
```

This adds the handlers to the router with the correct event names. Note that we can
handle more than one event with the same handler, if we'd like.
