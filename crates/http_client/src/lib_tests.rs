use prost::Message;

use super::*;

#[derive(Clone, PartialEq, Message)]
struct Utf8Proto {
    #[prost(string, tag = "1")]
    value: String,
}

#[derive(Clone, PartialEq, Message)]
struct BinaryProto {
    #[prost(bytes = "vec", tag = "1")]
    value: Vec<u8>,
}

#[test]
fn proto_request_preserves_utf8_body_without_serialized_payload() {
    let proto = Utf8Proto {
        value: "hello".to_string(),
    };
    let expected_bytes = proto.encode_to_vec();

    let request = Client::new()
        .post("https://example.com/proto")
        .proto(&proto)
        .build()
        .unwrap();

    assert_eq!(
        request.wrapped.headers().get(http::header::CONTENT_TYPE),
        Some(&HeaderValue::from_static("application/x-protobuf"))
    );
    assert_eq!(
        request.wrapped.body().and_then(reqwest::Body::as_bytes),
        Some(expected_bytes.as_slice())
    );
    assert_eq!(request.serialized_payload, None);
}

#[test]
fn proto_request_preserves_binary_body_without_serialized_payload() {
    let proto = BinaryProto {
        value: vec![0xff, 0xfe, 0xfd],
    };
    let expected_bytes = proto.encode_to_vec();

    let request = Client::new()
        .post("https://example.com/proto")
        .proto(&proto)
        .build()
        .unwrap();

    assert_eq!(
        request.wrapped.body().and_then(reqwest::Body::as_bytes),
        Some(expected_bytes.as_slice())
    );
    assert_eq!(request.serialized_payload, None);
}
