fn main() {
    let build = vergen::BuildBuilder::default()
        .build_timestamp(true)
        .build()
        .unwrap();
    vergen::Emitter::new()
        .add_instructions(&build)
        .unwrap()
        .emit()
        .unwrap();
}
