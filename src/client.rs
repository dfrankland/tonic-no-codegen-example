use protobuf::reflect::ReflectValueBox;
use protobuf_message_tonic_codec::RustProtobufMessageDynCodec;
use shared::get_protos;
use tonic::{transport::Channel, GrpcMethod};
use tower::ServiceBuilder;

mod protobuf_message_tonic_codec;
mod shared;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let protos = get_protos();
    let hello_request_descriptor = protos
        .message_by_package_relative_name("HelloRequest")
        .unwrap();
    let hello_reply_descriptor = protos
        .message_by_package_relative_name("HelloReply")
        .unwrap();

    let channel = Channel::from_static("http://[::1]:50051").connect().await?;

    let channel = ServiceBuilder::new().service(channel);

    let mut client = tonic::client::Grpc::new(channel);

    let mut hello_request = hello_request_descriptor.new_instance();
    let name_field = hello_request_descriptor.field_by_name("name").unwrap();
    name_field.set_singular_field(&mut *hello_request, ReflectValueBox::String("Tonic".into()));

    let mut request = tonic::Request::new(hello_request);
    // Request path pattern `/{mypackage}.{MyService}/{MyMethod}`
    request
        .extensions_mut()
        .insert(GrpcMethod::new("helloworld.Greeter", "SayHello"));
    let path = http::uri::PathAndQuery::from_static("/helloworld.Greeter/SayHello");

    client.ready().await.unwrap();
    let response = client
        .unary(
            request,
            path,
            RustProtobufMessageDynCodec(hello_reply_descriptor.clone()),
        )
        .await?;

    println!("RESPONSE={:?}", response);

    let response_message = &*response.into_inner();
    let message_field = hello_reply_descriptor.field_by_name("message").unwrap();
    let message_field_value = message_field.get_singular_field_or_default(response_message);

    println!("HelloReply.message=\"{}\"", message_field_value);

    Ok(())
}
