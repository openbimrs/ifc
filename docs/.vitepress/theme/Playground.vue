<script setup lang="ts">
// The browser playground (#331): `@openbim/ifc/web` reads a local .ifc in
// the page. Nothing is uploaded; the only requests are for the module and
// the samples, from this site. The module is not part of the VitePress
// build: the Pages workflow copies it to /playground-files/pkg/ after the build
// (docs/scripts/playground.mjs), so it is imported at runtime by URL.
import { withBase } from 'vitepress'
import { computed, nextTick, onBeforeUnmount, onMounted, ref, shallowRef } from 'vue'

import { buildScene, colourOf } from '../../../crates/openbim-ifc-wasm/examples/viewer/scene.mjs'
import { createViewer } from '../../../crates/openbim-ifc-wasm/examples/viewer/render.mjs'

type Value = { kind: string; value?: unknown; items?: Value[]; id?: bigint; type?: string }
type Row = { id: bigint; depth: number; label: string; container: boolean }

const SAMPLES = ['synthetic_properties.ifc', 'binding_geometry.ifc']
const MAX_FINDINGS = 500

const moduleState = ref<'loading' | 'ready' | 'missing'>('loading')
const version = ref('')
const status = ref('')
const error = ref('')
const fileName = ref('')
const header = shallowRef<Record<string, unknown> | null>(null)
const rows = shallowRef<Row[]>([])
const orphans = shallowRef<Row[]>([])
const selected = ref<bigint | null>(null)
const selectedLabel = ref('')
const sets = shallowRef<any[]>([])
const setsError = ref('')
const report = shallowRef<any | null>(null)
const meshStatus = ref('')
const meshesDrawn = ref<number | null>(null)
const canvas = ref<HTMLCanvasElement | null>(null)
const showCanvas = ref(false)

// Not reactive: wasm objects and bytes stay out of Vue's proxies.
let api: any = null
let meshApi: any = null
let model: any = null
let bytes: Uint8Array | null = null
let viewer: any = null

const loaded = computed(() => header.value !== null)

function plain(value: Value | undefined): string {
  if (!value) return ''
  switch (value.kind) {
    case 'typed':
      return plain(value.value as Value)
    case 'list':
      return (value.items ?? []).map(plain).join(', ')
    case 'ref':
      return `#${value.id}`
    case 'null':
    case 'derived':
    case 'unknown':
      return '—'
    default:
      return String(value.value)
  }
}

function label(id: bigint): string {
  let name = ''
  try {
    name = plain(model.attributeByName(id, 'Name'))
  } catch {
    // An entity without a Name attribute.
  }
  return `${model.typeOf(id)} #${id}${name && name !== '—' ? ` ${name}` : ''}`
}

function headerFields(h: any): [string, string][] {
  return [
    ['Schema', h.schema.join(', ')],
    ['Name', h.name],
    ['Time stamp', h.timeStamp],
    ['Author', h.author.join(', ')],
    ['Organization', h.organization.join(', ')],
    ['Originating system', h.originatingSystem],
    ['Preprocessor', h.preprocessorVersion],
    ['Description', h.description.join(', ')],
  ]
}

function flatten(tree: any): void {
  const byId = new Map<bigint, any>(tree.nodes.map((node: any) => [node.id, node]))
  const out: Row[] = []
  const visit = (id: bigint, depth: number) => {
    const node = byId.get(id)
    if (!node) return
    out.push({ id, depth, container: true, label: `${node.typeName} #${id}${node.name ? ` ${node.name}` : ''}` })
    for (const element of node.elements) out.push({ id: element, depth: depth + 1, container: false, label: label(element) })
    for (const child of node.children) visit(child, depth + 1)
  }
  for (const root of tree.roots) visit(root, 0)
  rows.value = out
  orphans.value = tree.orphans.map((id: bigint) => ({ id, depth: 0, container: false, label: label(id) }))
}

function select(id: bigint, text: string): void {
  selected.value = id
  selectedLabel.value = text
  setsError.value = ''
  try {
    sets.value = model.propertySets(id)
  } catch (failure: any) {
    sets.value = []
    setsError.value = `${failure.code ?? 'error'}: ${failure.message}`
  }
}

function open(data: Uint8Array, name: string): void {
  error.value = ''
  report.value = null
  meshStatus.value = ''
  meshesDrawn.value = null
  showCanvas.value = false
  selected.value = null
  sets.value = []
  try {
    const next = api.IfcModel.parse(data)
    model?.free()
    model = next
  } catch (failure: any) {
    error.value = `${name} could not be read: ${failure.code ?? 'error'}: ${failure.message}`
    return
  }
  bytes = data
  fileName.value = name
  header.value = model.header()
  try {
    flatten(model.spatialTree())
  } catch (failure: any) {
    rows.value = []
    orphans.value = []
    error.value = `No spatial tree: ${failure.code ?? 'error'}: ${failure.message}`
  }
  status.value = `${name}: ${model.schema ?? 'no schema'}, ${model.size} entities`
}

async function onFile(event: Event): Promise<void> {
  const [file] = (event.target as HTMLInputElement).files ?? []
  if (file) open(new Uint8Array(await file.arrayBuffer()), file.name)
}

async function onSample(event: Event): Promise<void> {
  const name = (event.target as HTMLSelectElement).value
  if (!name) return
  const response = await fetch(withBase(`/playground-files/samples/${name}`))
  if (!response.ok) {
    error.value = `The sample ${name} is not part of this build.`
    return
  }
  open(new Uint8Array(await response.arrayBuffer()), name)
}

function validate(): void {
  try {
    report.value = model.validate(MAX_FINDINGS)
  } catch (failure: any) {
    error.value = `Validation refused: ${failure.code ?? 'error'}: ${failure.message}`
  }
}

async function meshes(): Promise<void> {
  if (!bytes) return
  meshStatus.value = 'Loading the mesh module…'
  try {
    if (!meshApi) {
      meshApi = await import(/* @vite-ignore */ withBase('/playground-files/pkg/mesh/web/openbim_ifc_wasm.js'))
      await meshApi.default()
    }
    // The mesh entry is its own module instance: parse the bytes again there.
    const meshModel = meshApi.IfcModel.parse(bytes)
    let scene
    try {
      scene = buildScene(meshModel.productMeshes())
    } finally {
      meshModel.free()
    }
    meshesDrawn.value = scene.drawn.length
    const refused = scene.refused.map((r: any) => `#${r.id} ${r.code}`).join(', ')
    meshStatus.value = `${scene.drawn.length} meshes` + (refused ? `; refused: ${refused}` : '')
    showCanvas.value = scene.drawn.length > 0
    if (!showCanvas.value) return
    await nextTick()
    try {
      viewer ??= createViewer(canvas.value!)
      viewer.show(scene, colourOf)
    } catch (failure: any) {
      meshStatus.value += ` (not drawn: ${failure.message})`
    }
  } catch (failure: any) {
    meshStatus.value = `Meshes failed: ${failure.code ?? 'error'}: ${failure.message}`
  }
}

function redraw(): void {
  viewer?.draw()
}

onMounted(async () => {
  addEventListener('resize', redraw)
  try {
    api = await import(/* @vite-ignore */ withBase('/playground-files/pkg/web/openbim_ifc_wasm.js'))
    await api.default()
    moduleState.value = 'ready'
    try {
      version.value = (await (await fetch(withBase('/playground-files/pkg/package.json'))).json()).version
    } catch {
      // The version is decoration.
    }
  } catch {
    moduleState.value = 'missing'
  }
})

onBeforeUnmount(() => {
  removeEventListener('resize', redraw)
  model?.free()
  model = null
})
</script>

<template>
  <div class="playground">
    <p v-if="moduleState === 'loading'" class="note">Loading the WebAssembly module…</p>
    <p v-else-if="moduleState === 'missing'" class="note warning" data-testid="missing">
      This build of the site does not carry the playground's module. The Pages
      workflow builds it from the repository; locally, build the npm package and
      run <code>node docs/scripts/playground.mjs crates/openbim-ifc-wasm/pkg docs/public</code>
      before <code>npm run docs:dev</code>.
    </p>
    <template v-else>
      <div class="controls">
        <label class="picker">
          <span>Open a local .ifc file</span>
          <input type="file" accept=".ifc,.IFC" data-testid="file" @change="onFile" />
        </label>
        <label class="picker">
          <span>or a sample</span>
          <select data-testid="sample" @change="onSample">
            <option value="">choose…</option>
            <option v-for="sample in SAMPLES" :key="sample" :value="sample">{{ sample }}</option>
          </select>
        </label>
      </div>
      <p class="note">
        The file is read by <code>@openbim/ifc</code><span v-if="version"> {{ version }}</span> in this page;
        nothing is uploaded.
      </p>
      <p v-if="error" class="note warning" data-testid="error">{{ error }}</p>
      <p v-if="status" class="status" data-testid="status">{{ status }}</p>
    </template>

    <template v-if="loaded">
      <h2 id="playground-header">Header</h2>
      <table data-testid="header">
        <tbody>
          <tr v-for="[name, value] in headerFields(header)" :key="name">
            <th>{{ name }}</th>
            <td>{{ value }}</td>
          </tr>
        </tbody>
      </table>

      <div class="columns">
        <section>
          <h2 id="playground-tree">Spatial tree</h2>
          <ul class="tree" data-testid="tree">
            <li v-for="row in rows" :key="`${row.container}-${row.id}`" :style="{ paddingLeft: `${row.depth * 1.1}em` }">
              <button
                :class="{ container: row.container, active: selected === row.id }"
                @click="select(row.id, row.label)"
              >{{ row.label }}</button>
            </li>
            <li v-if="rows.length === 0" class="note">No spatial structure.</li>
          </ul>
          <template v-if="orphans.length">
            <h3>Not in the spatial structure</h3>
            <ul class="tree">
              <li v-for="row in orphans" :key="`orphan-${row.id}`">
                <button :class="{ active: selected === row.id }" @click="select(row.id, row.label)">{{ row.label }}</button>
              </li>
            </ul>
          </template>
        </section>
        <section>
          <h2 id="playground-properties">Property sets</h2>
          <p v-if="selected === null" class="note">Select an element or container in the tree.</p>
          <template v-else>
            <p class="status">{{ selectedLabel }}</p>
            <p v-if="setsError" class="note warning">{{ setsError }}</p>
            <p v-else-if="sets.length === 0" class="note">No property sets.</p>
            <div v-for="set in sets" :key="`${set.source}-${set.id}`" data-testid="pset">
              <h3>{{ set.name }} <small>({{ set.source === 'type' ? 'from its type' : 'its own' }})</small></h3>
              <table>
                <tbody>
                  <tr v-for="property in set.properties" :key="String(property.id)">
                    <th>{{ property.name }}</th>
                    <td>{{ plain(property.value) }}</td>
                  </tr>
                </tbody>
              </table>
            </div>
          </template>
        </section>
      </div>

      <h2 id="playground-validation">Validation</h2>
      <button class="action" data-testid="validate" @click="validate">Validate against {{ header?.schema?.[0] }}</button>
      <template v-if="report">
        <p class="status" data-testid="report">
          {{ report.conformant ? 'Conformant' : 'Not conformant' }}: {{ report.errors }} error(s),
          {{ report.evaluationErrors }} evaluation error(s), {{ report.warnings }} warning(s),
          {{ report.unsupported }} rule(s) not evaluated<span v-if="report.truncated">; the first {{ MAX_FINDINGS }} findings</span>.
        </p>
        <table v-if="report.findings.length" data-testid="findings">
          <thead>
            <tr><th>Severity</th><th>Rule</th><th>Entity</th><th>Message</th></tr>
          </thead>
          <tbody>
            <tr v-for="(finding, index) in report.findings" :key="index">
              <td>{{ finding.severity }}</td>
              <td><code>{{ finding.rule }}</code></td>
              <td>
                <button v-if="finding.entity !== undefined" class="link" @click="select(finding.entity, label(finding.entity))">#{{ finding.entity }}</button>
                <span v-else>file</span>
                <span v-if="finding.attributeName"> .{{ finding.attributeName }}</span>
              </td>
              <td>{{ finding.message }}</td>
            </tr>
          </tbody>
        </table>
      </template>

      <h2 id="playground-meshes">Meshes</h2>
      <p class="note">
        Meshes come from the <code>@openbim/ifc/mesh</code> entry, a second, larger module
        (about 5 MB) fetched on request.
      </p>
      <button class="action" data-testid="meshes" @click="meshes">Show meshes</button>
      <p v-if="meshStatus" class="status" data-testid="mesh-status" :data-drawn="meshesDrawn ?? ''">{{ meshStatus }}</p>
      <canvas v-show="showCanvas" ref="canvas" class="viewer"></canvas>
    </template>
  </div>
</template>

<style scoped>
.playground {
  margin-top: 1rem;
}
.controls {
  display: flex;
  flex-wrap: wrap;
  gap: 1rem 2rem;
  align-items: end;
}
.picker {
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
  font-size: 0.9em;
}
.picker select,
.picker input {
  font: inherit;
}
.picker select {
  border: 1px solid var(--vp-c-divider);
  border-radius: 4px;
  padding: 0.2rem 0.4rem;
  background: var(--vp-c-bg-soft);
}
.note {
  color: var(--vp-c-text-2);
  font-size: 0.9em;
}
.warning {
  color: var(--vp-c-warning-1);
}
.status {
  font-weight: 500;
}
.columns {
  display: grid;
  grid-template-columns: minmax(0, 1fr) minmax(0, 1fr);
  gap: 1.5rem;
}
@media (max-width: 720px) {
  .columns {
    grid-template-columns: minmax(0, 1fr);
  }
}
.tree {
  list-style: none;
  padding: 0;
  margin: 0.5rem 0;
  max-height: 32rem;
  overflow: auto;
}
.tree li {
  margin: 0;
}
.tree button,
button.link {
  font: inherit;
  font-size: 0.9em;
  text-align: left;
  color: var(--vp-c-text-1);
  padding: 0.1rem 0.3rem;
  border-radius: 4px;
}
.tree button.container {
  font-weight: 600;
}
.tree button:hover,
button.link:hover {
  color: var(--vp-c-brand-1);
}
.tree button.active {
  background: var(--vp-c-brand-soft);
}
button.action {
  border: 1px solid var(--vp-c-brand-1);
  color: var(--vp-c-brand-1);
  border-radius: 6px;
  padding: 0.3rem 0.9rem;
  font-weight: 500;
}
button.action:hover {
  background: var(--vp-c-brand-soft);
}
.viewer {
  display: block;
  width: 100%;
  height: 28rem;
  margin-top: 1rem;
  border: 1px solid var(--vp-c-divider);
  border-radius: 6px;
  touch-action: none;
}
</style>
