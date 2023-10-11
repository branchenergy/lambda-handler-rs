use std::any::TypeId;
use std::collections::HashMap;
use std::error::Error;
use std::fmt::Debug;
use std::future::Future;
use std::task::{Context, Poll};

use lambda_runtime::{Error as LambdaError, LambdaEvent};
use serde::Serialize;
use serde_json::Value;
use tower::Service;

use crate::events::{
    AwsEvent, AwsEventHandler, Callable, LambdaFuture, S3Event, SnsEvent, SqsEvent,
};

/// Parse the AWS Lambda event and return its type and name as a key.
///
/// # Arguments
///
/// * `event` - The AWS Lambda event to parse.
///
/// # Returns
///
/// A `Result` containing a tuple `(TypeId, String)` that represents
/// the event's type and name, or an error if parsing fails.
fn get_event_key(
    event: LambdaEvent<Value>,
) -> Result<(TypeId, String), Box<dyn Error + Send + Sync>> {
    // Attempt to parse as S3 event
    if let Ok(parsed_event) = S3Event::from_request(&event.payload) {
        return Ok((TypeId::of::<S3Event>(), parsed_event.event_name()));
    }

    // Attempt to parse as SNS event
    if let Ok(parsed_event) = SnsEvent::from_request(&event.payload) {
        return Ok((TypeId::of::<SnsEvent>(), parsed_event.event_name()));
    }

    // Attempt to parse as SQS event
    if let Ok(parsed_event) = SqsEvent::from_request(&event.payload) {
        return Ok((TypeId::of::<SqsEvent>(), parsed_event.event_name()));
    }

    Err("Failed to parse event".into())
}

/// Router for AWS Lambda functions.
///
/// Routes incoming AWS events to their corresponding handlers based
/// on their types and names.
pub struct LambdaHandler<R>
where
    R: Debug + Serialize + 'static,
{
    // Mapping of event type and name to its handler
    handlers: HashMap<(TypeId, String), Box<dyn Callable<R>>>,
}

impl<R: Debug + Serialize> LambdaHandler<R> {
    /// Creates a new `LambdaHandler`.
    ///
    /// # Returns
    ///
    /// A new `LambdaHandler` instance.
    pub fn new() -> Self {
        LambdaHandler {
            handlers: HashMap::new(),
        }
    }

    /// Adds a route to the router.
    ///
    /// # Arguments
    ///
    /// * `event_name` - The name of the AWS event to route.
    /// * `handler` - The function to handle the routed event.
    ///
    /// # Returns
    ///
    /// The router itself, allowing for method chaining.
    pub fn route<E, F, Fut>(mut self, event_name: &str, handler: F) -> Self
    where
        E: AwsEvent + 'static,
        F: Fn(LambdaEvent<E>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<R, LambdaError>> + Send + 'static,
    {
        let boxed_event_handler = Box::new(AwsEventHandler::new(Box::new(move |event| {
            Box::pin(handler(event)) as LambdaFuture<R>
        })));

        self.handlers.insert(
            (TypeId::of::<E>(), event_name.to_string()),
            boxed_event_handler,
        );
        self
    }
}

impl<R: Debug + Serialize> Default for LambdaHandler<R> {
    /// Provides default instance of `LambdaHandler`.
    ///
    /// # Returns
    ///
    /// A default `LambdaHandler` instance.
    fn default() -> Self {
        Self::new()
    }
}

impl<R: Debug + Serialize> Service<LambdaEvent<Value>> for LambdaHandler<R> {
    type Response = R;
    type Error = LambdaError;
    type Future = LambdaFuture<R>;

    /// Checks if the service is ready to process a request.
    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    /// Processes an incoming event, routing it to the appropriate handler.
    ///
    /// # Arguments
    ///
    /// * `req` - The incoming AWS event.
    ///
    /// # Returns
    ///
    /// A future resolving to the response of the handler, or a Lambda error.
    fn call(&mut self, req: LambdaEvent<Value>) -> Self::Future {
        let cloned_request = req.clone();
        let event_key = get_event_key(cloned_request);

        match event_key {
            Ok(key) => {
                if let Some(handler) = self.handlers.get(&key) {
                    handler.call(req)
                } else {
                    Box::pin(async {
                        Err(LambdaError::from("I don't have a handler for this event!"))
                    })
                }
            }
            Err(_) => {
                Box::pin(async { Err(LambdaError::from("Unable to get event type or name!")) })
            }
        }
    }
}
