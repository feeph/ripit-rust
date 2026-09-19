/*
    PRGC - title progress (current)

```TEXT
PRGC:<code>,<id>,<name>
```

- `code`: unique message code
- `id`: operation sub-id
- `name`: name string
*/

#[derive(Clone, Debug, PartialEq)]
pub struct ProgressCurrentRecord {
    pub code: u32,
    pub id: u32,
    pub name: String,
}

impl ProgressCurrentRecord {
    pub fn new(code: u32, id: u32, name: &str) -> Self {
        Self { code, id, name: name.to_owned() }
    }
}

pub fn parse_prgc_data(data: &[u8]) -> (u32, u32, String) {
    use crate::api::line_parser::parse_line;
    let fields = parse_line(data);

    let code = fields[0].parse::<u32>().unwrap();
    let id = fields[1].parse::<u32>().unwrap();
    let name = fields[2].to_string();

    (code, id, name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progress_current_new() {
        // ----------------------------------------------------------------
        let computed = ProgressCurrentRecord::new(3120, 1, "Scanning contents");
        let expected = ProgressCurrentRecord {
            code: 3120,
            id: 1,
            name: "Scanning contents".to_string(),
        };
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }
}