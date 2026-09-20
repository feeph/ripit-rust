/*
    PRGT - title progress (total)

```TEXT
PRGT:<code>,<id>,<name>
```

- `code`: unique message code
- `id`: operation sub-id
- `name`: name string
*/

#[derive(Clone, Debug, PartialEq)]
pub struct ProgressTotalRecord {
    pub source: String,
    pub code: u32,
    pub id: u32,
    pub name: String,
}

impl ProgressTotalRecord {
    pub fn new(source: &str, code: u32, id: u32, name: String) -> Self {
        Self {
            source: source.to_string(),
            code,
            id,
            name: name.to_owned(),
        }
    }
}

pub fn parse_prgt_data(data: &[u8]) -> (u32, u32, String) {
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
    fn test_progress_total_new() {
        let source = "source_name".to_string();
        let name = "Scanning CD-ROM devices".to_string();
        // ----------------------------------------------------------------
        let computed = ProgressTotalRecord::new(&source, 5018, 0, name.clone());
        let expected = ProgressTotalRecord {
            source,
            code: 5018,
            id: 0,
            name,
        };
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }
}
