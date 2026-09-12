use std::marker::PhantomData;
use windmark::{context::RouteContext, response::Response};

#[rossweisse::router(value = T::default())]
#[derive(Clone)]
struct Capsule<'a, T: Default + Clone, const SIZE: usize = 2> {
  value:       T,
  label:       &'a str,
  marker:      PhantomData<[(); SIZE]>,
  #[cfg(any())]
  absent:      Missing,
  #[cfg_attr(all(), cfg(any()))]
  also_absent: Missing,
}

#[rossweisse::router]
impl<'a, T: Default + Clone, const SIZE: usize> Capsule<'a, T, SIZE> {
  #[rossweisse::route(index)]
  fn index(_: RouteContext) -> Response { Response::success("index") }

  #[cfg_attr(all(), cfg(any()))]
  #[rossweisse::route]
  fn absent(_: Missing) -> Missing { unreachable!() }
}

#[rossweisse::router]
#[cfg_attr(all(), cfg(any()), derive(Clone))]
struct Absent;

#[rossweisse::router]
#[cfg(any())]
impl Absent {}

fn main() {
  let capsule = Capsule::<u8>::new().clone();

  assert_eq!(capsule.value, 0);
  assert_eq!(capsule.label, "");
}
