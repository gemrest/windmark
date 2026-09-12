mod capsule {
  #[rossweisse::router]
  pub struct Capsule;

  #[rossweisse::router]
  pub(crate) struct Stateful {
    visits: usize,
  }

  mod nested {
    #[rossweisse::router]
    pub(super) struct ParentVisible;

    #[rossweisse::router]
    pub(in crate::capsule) struct Scoped;
  }

  pub fn check_nested_visibility() {
    let _: Option<nested::ParentVisible> = None;
    let _: Option<nested::Scoped> = None;
  }
}

fn main() {
  let _: Option<capsule::Capsule> = None;
  let _: Option<capsule::Stateful> = None;

  capsule::check_nested_visibility();
}
