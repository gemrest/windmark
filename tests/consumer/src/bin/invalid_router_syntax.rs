macro_rules! invalid_declaration {
  () => {
    #[rossweisse::router]
    struct Capsule { value: , }
  };
}

invalid_declaration!();

fn main() {}
