//! The lossless tagged value encoding (ADR 0013).
//!
//! Every IFC attribute value crosses to JavaScript as an object with a `kind`
//! field, plus a payload field that depends on the kind:
//!
//! | `kind`    | payload                 | STEP example           |
//! | --------- | ----------------------- | ---------------------- |
//! | `null`    | —                       | `$`                    |
//! | `derived` | —                       | `*`                    |
//! | `bool`    | `value: boolean`        | `.T.` / `.F.`          |
//! | `unknown` | —                       | `.U.`                  |
//! | `integer` | `value: bigint`         | `42`                   |
//! | `real`    | `value: number`         | `2.5`                  |
//! | `text`    | `value: string`         | `'Wall'`               |
//! | `binary`  | `value: string` (hex)   | `"0123ABC"`            |
//! | `enum`    | `value: string`         | `.ELEMENT.`            |
//! | `ref`     | `id: bigint`            | `#42`                  |
//! | `list`    | `items: Value[]`        | `(1,2)`                |
//! | `typed`   | `type: string, value: Value` | `IFCLABEL('x')`   |
//!
//! Integers and ids are `bigint` because IFC integers are 64-bit and a JS
//! number loses precision above 2^53. `unknown` is its own kind rather than
//! `bool` with a null value so that no JS truthiness check can confuse it
//! with `.F.`.
//!
//! The table names the JavaScript fields; Python and C carry the same kinds
//! and payloads in their own idiom. [`Tagged`] is the host-independent form:
//! each binding only moves `Tagged` into and out of host values, so the
//! mapping itself is tested once, natively.

mod tagged;

pub use tagged::{Kind, Tagged};

/// Deepest list or typed-wrapper nesting accepted from a host.
///
/// Matches the STEP parser's limit, so any value the parser can produce can
/// round-trip, while a hostile deeply nested object cannot overflow the stack.
pub const MAX_NESTING: usize = 128;
