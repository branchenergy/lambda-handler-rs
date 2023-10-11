use std::marker::PhantomData;
use std::pin::Pin;
use std::{fmt::Debug, future::Future};

use lambda_runtime::{Error as LambdaError, LambdaEvent};
use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;

pub use aws_lambda_events::{s3::S3Event, sns::SnsEvent, sqs::SqsEvent};

/// Trait defining methods that AWS events must implement.
pub trait AwsEvent: Send + Sync + Sized + DeserializeOwned {
    /// Deserializes an AWS event from a JSON request.
    fn from_request(request: &Value) -> Result<Self, serde_json::Error> {
        serde_json::from_value(request.clone())
    }

    /// Converts a generic `LambdaEvent` into a specialized AWS event.
    fn from_event(event: LambdaEvent<Value>) -> Result<LambdaEvent<Self>, serde_json::Error> {
        let deserialised: Self = Self::from_request(&event.payload)?;
        Ok(LambdaEvent {
            payload: deserialised,
            context: event.context,
        })
    }

    /// Returns the event name for the AWS event.
    fn event_name(&self) -> String;
}

// Implement `AwsEvent` trait for `S3Event`.
impl AwsEvent for S3Event {
    fn event_name(&self) -> String {
        self.records[0].event_name.clone().expect("No event name!")
    }
}

// Implement `AwsEvent` trait for `SnsEvent`.
impl AwsEvent for SnsEvent {
    fn event_name(&self) -> String {
        self.records[0].sns.topic_arn.clone()
    }
}

// Implement `AwsEvent` trait for `SqsEvent`.
impl AwsEvent for SqsEvent {
    fn event_name(&self) -> String {
        self.records[0]
            .event_source_arn
            .clone()
            .expect("No topic ARN!")
    }
}

/// Type alias for a Lambda future, wrapping a boxed dynamic Future trait.
pub type LambdaFuture<R> = Pin<Box<dyn Future<Output = Result<R, LambdaError>> + Send>>;

/// Type alias for a Lambda function handler.
pub type LambdaFn<R> = Box<dyn Fn(LambdaEvent<Box<dyn AwsEvent>>) -> LambdaFuture<R> + Send + Sync>;

/// Trait for objects that can be called within a Lambda function.
pub trait Callable<R> {
    /// Method to handle the AWS Lambda event and produce a future.
    fn call(&self, event: LambdaEvent<Value>) -> LambdaFuture<R>;
}

/// Generic struct for AWS Event Handlers.
pub struct AwsEventHandler<T, R>
where
    T: AwsEvent,
    R: Debug + Serialize,
{
    event_type: PhantomData<T>, // Marker for the event type.
    handler: Box<dyn Fn(LambdaEvent<T>) -> LambdaFuture<R> + Send + Sync>, // Actual event handler function.
}

// Implementation for `AwsEventHandler`.
impl<T: AwsEvent, R: Debug + Serialize> AwsEventHandler<T, R> {
    /// Constructs a new `AwsEventHandler`.
    pub fn new(handler: Box<dyn Fn(LambdaEvent<T>) -> LambdaFuture<R> + Send + Sync>) -> Self {
        AwsEventHandler {
            event_type: PhantomData,
            handler,
        }
    }
}

// Implement `Callable` trait for `AwsEventHandler`.
impl<T: AwsEvent, R: Debug + Serialize> Callable<R> for AwsEventHandler<T, R> {
    /// Calls the event handler for an AWS Lambda event.
    fn call(&self, event: LambdaEvent<Value>) -> LambdaFuture<R> {
        // Convert the generic Request into a specific event type.
        let res = T::from_event(event);

        // If the conversion is successful, call the handler, else return an error.
        match res {
            Ok(converted_event) => (self.handler)(converted_event),
            Err(_) => Box::pin(async { Err(LambdaError::from("Failed to process event!")) }),
        }
    }
}
