use protobuf::reflect::FileDescriptor;
use protobuf_parse::Parser;

pub fn get_protos() -> FileDescriptor {
    let mut parser = Parser::new();
    parser.protoc();
    parser.include(env!("CARGO_MANIFEST_DIR"));
    parser.input(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/proto/helloworld.proto"
    ));
    let mut fds = parser.file_descriptor_set().unwrap();
    FileDescriptor::new_dynamic(fds.file.pop().unwrap(), &[]).unwrap()
}
