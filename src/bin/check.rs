use hickory_proto::op::{Message, MessageType, OpCode};
fn main() {
    let msg = Message::new(0, MessageType::Query, OpCode::Query);
    println!("{:#?}", msg);
}
