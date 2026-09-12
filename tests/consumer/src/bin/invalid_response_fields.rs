fn main() {
  let mut response = windmark::response::Response::success("hello");

  response.status = 21;
}
