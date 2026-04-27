use lithos_protocol::codec;
use lithos_protocol::messages::ClientMessage;

/// Bytes produced by `encode({ Join: { token: 'abc' } })` from `@msgpack/msgpack` in the browser client.
const JS_CLIENT_JOIN_ABC: &[u8] = &[
    0x81, 0xa4, 0x4a, 0x6f, 0x69, 0x6e, 0x81, 0xa5, 0x74, 0x6f, 0x6b, 0x65, 0x6e, 0xa3, 0x61, 0x62,
    0x63,
];

#[test]
fn decode_js_encoded_client_join() {
    let msg: ClientMessage = codec::decode(JS_CLIENT_JOIN_ABC).unwrap();
    assert_eq!(
        msg,
        ClientMessage::Join {
            token: "abc".into(),
        }
    );
}
