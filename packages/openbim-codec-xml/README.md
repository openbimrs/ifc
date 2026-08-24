# openbim-codec-xml

Container-level XML recognition: BOM handling and content sniffing.

It exists as its own crate because both the IFC codecs and the openBIM standards read XML, and the IFC layer must never depend on the openBIM layer — so the shared piece sits below both.

## Scope: recognition, not parsing

This crate answers *"what is this byte stream?"* — nothing more. Element trees,
archive entry extraction and schema binding belong to the format crates.

The boundary is deliberate: a shared "utilities" crate has no natural stopping
point and ends up absorbing every format's quirks.

## Detect by content, never by extension

openBIM files are routinely misnamed in the wild, and the same extension
appears with different container shapes depending on which tool wrote the file.
Dispatching on the extension produces errors that read like corruption rather
than a wrong-container guess.

## Part of nehirde

A pure-Rust IFC and openBIM toolchain: <https://github.com/GeneralPawz/nehirde>

## License

MIT
