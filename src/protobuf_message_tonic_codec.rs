use bytes::{Buf, BufMut};
use protobuf::{reflect::MessageDescriptor, MessageDyn};
use tonic::{
    codec::{Codec, DecodeBuf, Decoder, EncodeBuf, Encoder},
    Status,
};

#[derive(Clone)]
pub struct RustProtobufMessageDynCodec(pub MessageDescriptor);

impl Codec for RustProtobufMessageDynCodec {
    type Encode = Box<dyn MessageDyn>;
    type Decode = Box<dyn MessageDyn>;

    type Encoder = RustProtobufMessageDynEncoder;
    type Decoder = RustProtobufMessageDynDecoder;

    fn encoder(&mut self) -> Self::Encoder {
        RustProtobufMessageDynEncoder
    }

    fn decoder(&mut self) -> Self::Decoder {
        RustProtobufMessageDynDecoder(self.0.clone())
    }
}

#[derive(Default, Debug, Clone)]
pub struct RustProtobufMessageDynEncoder;

impl Encoder for RustProtobufMessageDynEncoder {
    type Item = Box<dyn MessageDyn>;
    type Error = Status;

    fn encode(&mut self, item: Self::Item, buf: &mut EncodeBuf<'_>) -> Result<(), Self::Error> {
        let required_size = item.compute_size_dyn();
        let required: usize = required_size.try_into().map_err(|_| {
            Status::failed_precondition(
                format!("Architecture does not have enough capacity to write message `{}` requested {required_size} bytes, but only {} maximum bytes supported", item.descriptor_dyn().full_name(), usize::MAX))
        })?;
        let remaining = buf.remaining_mut();
        if required > buf.remaining_mut() {
            return Err(Status::failed_precondition(format!("Provided buffer does not have enough capacity to write message `{}` requested {required_size} bytes, but only {remaining} remaining bytes", item.descriptor_dyn().full_name())));
        }

        item.write_to_writer_dyn(&mut buf.writer())
            .map_err(|error| Status::internal(error.to_string()))?;

        Ok(())
    }
}

#[derive(Clone)]
pub struct RustProtobufMessageDynDecoder(MessageDescriptor);

impl Decoder for RustProtobufMessageDynDecoder {
    type Item = Box<dyn MessageDyn>;
    type Error = Status;

    fn decode(&mut self, buf: &mut DecodeBuf<'_>) -> Result<Option<Self::Item>, Self::Error> {
        Ok(Some(
            self.0
                .parse_from_reader(&mut buf.reader())
                .map_err(|error| Status::internal(error.to_string()))?,
        ))
    }
}
