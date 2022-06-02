fn main() {
    tonic_build::compile_protos("proto/orderbook.proto").unwrap();
}
