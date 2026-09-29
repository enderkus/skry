fn main() {
    let nonce = std::env::args().nth(1).unwrap_or("N".into());
    let cmd = skry::collect::script::build(&skry::collect::script::Section::all(true), &nonce);
    print!("{cmd}");
}
