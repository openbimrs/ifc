---
outline: false
---

# Playground

<script setup>
import Playground from './.vitepress/theme/Playground.vue'
</script>

Open an IFC file and look inside it with
[`@openbim/ifc`](/bindings/javascript), the WebAssembly build of this
library, running in this page: the file header, the spatial tree, the
property sets of the element you select, a validation report, and the
meshes of every product with a Body. The file never leaves your browser.

<ClientOnly>
  <Playground />
</ClientOnly>

## How it works

The page imports the package's `web` build, built from `main` with this
site, and calls the same methods the [JavaScript cookbook](/cookbook/javascript)
shows: `IfcModel.parse`, `header()`, `spatialTree()`, `propertySets(id)`,
`validate()` and, from the `mesh` entry, `productMeshes()`. The meshes are
drawn by the WebGL viewer in `crates/openbim-ifc-wasm/examples/viewer/`.
The two samples are fixtures from the repository's `test/fixtures/`.
