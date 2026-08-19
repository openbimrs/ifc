# xml implementation plan

Status: implemented and verified.

## Work queue

- [x] `XML-NODE` - bounded internal node reader over quick-xml.
- [x] `XML-PSD` - all seven IFC4 PSD property forms including nested complex properties.
- [x] `XML-QTO` - all measured IFC4 quantity forms.
- [x] `XML-EDGE` - namespaces, empty aliases, applicability lists, malformed/unknown types.
- [x] `XML-CORPUS` - parse all 513 IFC4 files and prove 2,550/257 child counts.

## Completion log

Append concise proof and newly observed source quirks here.

- Unit tests cover all property/quantity forms, unknown types, and resource limits; generation parsed the full 513-file corpus.
