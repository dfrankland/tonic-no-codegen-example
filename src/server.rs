use protobuf::{
    reflect::{FileDescriptor, MethodDescriptor, ReflectValueBox},
    MessageDyn,
};
use protobuf_message_tonic_codec::RustProtobufMessageDynCodec;
use shared::get_protos;
use std::{convert::Infallible, time::Duration};
use tonic::{
    server::{NamedService, UnaryService},
    transport::Server,
};
use tower::Service;

mod protobuf_message_tonic_codec;
mod shared;

struct MethodUnarySvc {
    method: MethodDescriptor,
}

impl MethodUnarySvc {
    fn new(method: MethodDescriptor) -> Self {
        Self { method }
    }

    fn unary(
        self: &Self,
        req: tonic::Request<Box<dyn MessageDyn>>,
    ) -> tonic::codegen::BoxFuture<tonic::Response<Box<dyn MessageDyn>>, tonic::Status> {
        let input_type = self.method.input_type();
        let output_type = self.method.output_type();
        let mut output: Box<dyn MessageDyn> = output_type.new_instance();
        if let Some(message_field) = input_type.field_by_name("name") {
            let request_message = &*req.into_inner();
            let message_field_value = message_field.get_singular_field_or_default(request_message);
            if let Some(reply_field) = output_type.field_by_name("message") {
                reply_field.set_singular_field(
                    &mut *output,
                    ReflectValueBox::String(message_field_value.to_string()),
                );
            }
        }
        let fut = async move { Ok(tonic::Response::new(output)) };
        Box::pin(fut)
    }
}

impl UnaryService<Box<dyn MessageDyn>> for MethodUnarySvc {
    type Response = Box<dyn MessageDyn>;
    type Future = tonic::codegen::BoxFuture<tonic::Response<Self::Response>, tonic::Status>;

    fn call(&mut self, req: tonic::Request<Box<dyn MessageDyn>>) -> Self::Future {
        self.unary(req)
    }
}

fn unimplemented() -> tonic::codegen::BoxFuture<http::Response<tonic::body::BoxBody>, Infallible> {
    Box::pin(async move {
        Ok(http::Response::builder()
            .status(200)
            .header("grpc-status", "12")
            .header("content-type", "application/grpc")
            .body(tonic::codegen::empty_body())
            .unwrap())
    })
}

#[derive(Clone, Debug)]
struct Svc {
    file: FileDescriptor,
}

impl Svc {
    pub fn new() -> Self {
        Self { file: get_protos() }
    }
}

impl<B> Service<http::Request<B>> for Svc
where
    B: http_body::Body + Send + 'static,
    B::Error: Into<tonic::codegen::StdError> + Send + 'static,
{
    type Response = http::Response<tonic::body::BoxBody>;
    type Error = Infallible;
    type Future = tonic::codegen::BoxFuture<Self::Response, Self::Error>;

    fn poll_ready(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        Ok(()).into()
    }

    fn call(&mut self, req: http::Request<B>) -> Self::Future {
        // Find method based on request path pattern `/{mypackage}.{MyService}/{MyMethod}`
        let found_method = self
            .file
            .services()
            .find(|service| {
                req.uri().path().starts_with(&format!(
                    "/{}.{}",
                    self.file.package(),
                    service.proto().name()
                ))
            })
            .and_then(|service| {
                service.methods().find(|method| {
                    req.uri()
                        .path()
                        .ends_with(&format!("/{}", method.proto().name()))
                })
            });

        if let Some(method) = found_method {
            let input_type = method.input_type();
            let codec = RustProtobufMessageDynCodec(input_type);
            let mut grpc = tonic::server::Grpc::new(codec);

            let fut = async move {
                let unary_service = MethodUnarySvc::new(method);
                let res = grpc.unary(unary_service, req).await;
                Ok(res)
            };

            Box::pin(fut)
        } else {
            unimplemented()
        }
    }
}

impl NamedService for Svc {
    // Somewhat of a "hack" to allow this service to be called for any service requested.
    // https://docs.rs/axum/0.6.9/axum/routing/struct.Router.html#method.route
    const NAME: &'static str = ":svc";
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50051".parse().unwrap();

    println!("{} listening on {}", Svc::NAME, addr);

    let layer = tower::ServiceBuilder::new()
        .timeout(Duration::from_secs(30))
        .into_inner();

    Server::builder()
        .layer(layer)
        .add_service(Svc::new())
        .serve(addr)
        .await?;

    Ok(())
}
