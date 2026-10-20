use aip_160::parse_filter;

fn main() {
    let examples = [
        "name = \"John Doe\"",
        "state = SUSPENDED AND score >= 2.5",
        // OR binds tighter than AND: this is `a = 1 AND (b = 2 OR c = 3)`.
        "a = 1 AND b = 2 OR c = 3",
        // A whitespace sequence; exact-match services treat it as AND.
        "New York Giants",
        "-file:\".java\" AND NOT deleted = true",
        "labels.env = prod AND regex(name, \"^web-\")",
        "ttl < 1.5s AND email = \"*@example.com\"",
        "value = null",
        "a = = b",
    ];

    for example in examples {
        println!("{example}");
        match parse_filter(example) {
            Ok(filter) => {
                println!("  canonical: {filter}");
                println!("  ast: {:?}", filter.expression);
            }
            Err(error) => println!("  error: {error}"),
        }
        println!();
    }
}
