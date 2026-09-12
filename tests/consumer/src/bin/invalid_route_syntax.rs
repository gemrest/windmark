macro_rules! invalid_declaration {
  () => {
    #[rossweisse::route]
    fn route(value: ) {}
  };
}

invalid_declaration!();

fn main() {}
