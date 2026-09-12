mod capsule {
  pub mod nested {
    #[rossweisse::router]
    pub(in crate::capsule) struct Capsule;
  }
}

fn main() { let _: Option<capsule::nested::Capsule> = None; }
