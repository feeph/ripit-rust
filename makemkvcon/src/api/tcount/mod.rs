/*
    title count record (TCOUNT)

    (documented as 'TCOUT' in https://www.makemkv.com/developers/usage.txt)
*/

#[allow(unused_imports)]
use log::{debug, error, info, warn};

pub fn parse_title_count_data(data: &[u8]) -> usize {
    let s = std::str::from_utf8(data).unwrap();
    s.parse::<usize>().unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    // TCOUNT:12
    #[test]
    fn parse_title_count() {
        let data = b"12";
        // ----------------------------------------------------------------
        let computed = parse_title_count_data(data);
        let expected = 12;
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }

}
