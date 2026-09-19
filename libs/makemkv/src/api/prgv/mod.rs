/*
    PRGV - progress bar values (current & total)

```TEXT
PRGV:<current>,<total>,<max>
```

- `current`: current progress value
- `total`: total progress value
- `max`: maximum possible value for a progress bar, constant
*/

// third-party imports
#[allow(unused_imports)]
use log::{debug, error, info, trace, warn};

#[derive(Clone, Debug, PartialEq)]
pub struct ProgressValueRecord {
    pub current: u32,
    pub total: u32,
    pub maximum: u32,
}

pub fn parse_prgv_data(data: &[u8]) -> (u32, u32, u32) {
    use crate::api::line_parser::parse_line;
    let fields = parse_line(data);

    let current = fields[0].parse::<u32>().unwrap();
    let total = fields[1].parse::<u32>().unwrap();
    let maximum = fields[2].parse::<u32>().unwrap();

    // not supposed to trigger
    // (if this triggers there's an issue with MakeMKV)
    if current > maximum {
        warn!("PRGV: Current percentage exceeds maximum percentage! ({} > {})", current, maximum)
    }
    // current percentage may exceed total percentage
    if current > total {
        debug!("PRGV: Current percentage exceeds total percentage. ({} > {})", current, total)
    }
    if total > maximum {
        warn!("PRGV: Total percentage exceeds maximum percentage! ({} > {})", current, maximum)
    }

    (current, total, maximum)
}
