# CONTRIBUTING

- [Coding Conventions](#coding-conventions)
  - [Data Objects](#data-objects)
    - [Data Object Conventions](#data-object-conventions)
      - [Data Object Values](#data-object-values)
      - [Data Object Traits](#data-object-traits)
    - [Data Object Examples](#data-object-examples)
      - [Flat Data Object](#flat-data-object)
      - [Nested Data Object](#nested-data-object)
  - [Unit tests](#unit-tests)
    - [Unit test Conventions](#unit-test-conventions)
      - [Arrange / Act / Assert](#arrange--act--assert)
      - [Assert conditions](#assert-conditions)
      - [Structure](#structure)
    - [Unit Test Examples](#unit-test-examples)
      - [Unit test with empty arrange block](#unit-test-with-empty-arrange-block)
      - [Unit test with arrange block](#unit-test-with-arrange-block)

## Coding Conventions

This section documents the coding conventions used by this project.

- All contributed code is expected to follow these conventions.
- If something is not documented, imitate the structure of the existing code.

### Data Objects

#### Data Object Conventions

##### Data Object Values

All data object members are publicly accessible.

- Do not create setters/getters for data objects.
- If a getter/setter is necessary then this is a good indication that
  - the item in question shouldn't be a data object.
  - a refactoring should be considered.

```RUST
use crate::api::ProgressCurrentRecord;

let pcr = ProgressCurrentRecord::default();

debug!("code: {}", pcr.code);
debug!("id:   {}", pcr.id);
debug!("name: {}", pcr.name);
```

##### Data Object Traits

All data objects implement the traits 'Debug' and 'PartialEq' to allow for
easy printing and comparison of their values.

```RUST
let x = Settings::default();
let y = Settings::default();

// requires 'Debug'
debug!("value of x: {:#?}", x);
debug!("value of y: {:#?}", y);

// requires 'PartialEq'
if x == y {
    debug!("Both data objects have equal values.");
}
```

#### Data Object Examples

##### Flat Data Object

```RUST
#[derive(Debug, PartialEq)]
pub enum DirectIO {
    Enabled,
    Disabled,
}
```

##### Nested Data Object

```RUST
#[derive(Debug, PartialEq)]
pub struct DirectIO {
    <...>
}

#[derive(Debug, PartialEq)]
pub struct OutputType {
    <...>
}

#[derive(Debug, PartialEq)]
pub struct Settings {
    pub directio: DirectIO,
    pub messages: OutputType,
    pub progress: OutputType,
}
```

### Unit tests

#### Unit test Conventions

##### Arrange / Act / Assert

Code in test functions is expected to follow the pattern

1. **arrange** - prepare for testing
2. **act** - compute the test value
3. **assert** - confirm 'computed' and 'expected' output

To help with reading:

- clearly indicate 'computed' and 'expected' value in the act block
- group the code into three blocks, separated by a dashed line

##### Assert conditions

It is good practice to have exactly one assert condition per test.

##### Structure

```TEXT
<...>
    #[test]
    fn test_<name>() {
        <arrange block>
        // ----------------------------------------------------------------
        <act block>
        // ----------------------------------------------------------------
        <assert block>
    }
<...>
```

#### Unit Test Examples

##### Unit test with empty arrange block

```RUST
mod tests {
    use super::*;

    #[test]
    fn test_settings_default() {
        // ----------------------------------------------------------------
        let computed = Settings::default();
        let expected = Settings{
            directio: DirectIO::Enabled,
            messages: OutputType::StdOut,
            progress: OutputType::Silent,
        };
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }
}
```

##### Unit test with arrange block

```RUST
mod tests {
    use super::*;

    #[test]
    fn test_attr_id_to_string() {
        let data = ItemAttributeId::Bitrate;
        // ----------------------------------------------------------------
        let computed = data.to_string();
        let expected = "Bitrate".to_string();
        // ----------------------------------------------------------------
        assert_eq!(computed, expected);
    }
}
```
