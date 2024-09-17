use regex::Regex;
use std::sync::LazyLock;

static REGEX_NON_ALNUM: LazyLock<Regex> = LazyLock::new(|| Regex::new("[^a-zA-Z0-9]+").unwrap());

pub fn make_snake_case(s: &str) -> String {
    let mut iter = REGEX_NON_ALNUM.split(s);
    let mut result = iter.next().unwrap_or("").to_ascii_lowercase();
    for slice in iter {
        if slice.is_empty() {
            continue;
        }
        result.push('_');
        result.push_str(&slice.to_ascii_lowercase());
    }
    result
}

#[cfg(test)]
mod tests {
    use super::make_snake_case;

    #[test]
    fn test_make_snake_case() {
        let testcases = &[
            ("HELLO, World!", "hello_world"),
            ("upsuper-vps11", "upsuper_vps11"),
            (r"C:\Users", "c_users"),
        ];
        for (input, expected) in testcases {
            assert_eq!(make_snake_case(input), *expected);
        }
    }
}
