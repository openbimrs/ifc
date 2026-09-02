import { defineConfig } from 'vitepress'

import { diagramPlugin } from './diagram-plugin.ts'

const adrs = [
  ['0001-entity-graph-free-of-domain-and-codec', 1, 'Domain- and codec-free graph'],
  ['0002-codec-as-a-model-crate-trait', 2, 'Codec as a model-crate trait'],
  ['0003-domain-crates-as-borrowed-views', 3, 'Domain crates as borrowed views'],
  ['0004-geometry-bridge-not-kernel', 4, 'Geometry bridge, not kernel'],
  ['0005-scaffold-modules-declare-ownership', 5, 'Scaffold module honesty'],
  ['0006-facade-features-default-to-thin', 6, 'Thin default features'],
  ['0007-authoring-is-a-schema-layer-not-a-model-layer', 7, 'Authoring is a schema layer'],
  ['0008-fixed-slot-constants-for-stable-relationships', 8, 'Fixed slot constants'],
  ['0009-derived-attributes-resolve-through-the-parent-context', 9, 'DERIVED attribute inheritance'],
]

function adrItems() {
  return adrs.map(function (entry) {
    return { text: entry[2], link: '/adr/' + entry[0] }
  })
}

export default defineConfig({
  title: 'openbim-ifc',
  description: 'Pure-Rust IFC model, codecs, schema metadata, and typed domain projections.',
  lang: 'en-US',
  base: '/ifc/',
  cleanUrls: true,
  lastUpdated: true,
  // AGENTS.md / PLAN.md are agent context files beside the site sources, not
  // pages. Excluding them keeps the dead-link gate meaningful, since their
  // relative pointers target repository files rather than routes.
  srcExclude: ['**/AGENTS.md', '**/PLAN.md', 'adr/_template.md'],
  markdown: {
    html: false,
    math: true,
    config: diagramPlugin,
  },
  sitemap: { hostname: 'https://openbimrs.github.io/ifc/' },
  head: [
    ['meta', { name: 'theme-color', content: '#1d4ed8' }],
    ['meta', { name: 'robots', content: 'index,follow' }],
  ],
  themeConfig: {
    logo: '/logo.svg',
    siteTitle: 'openbim-ifc',
    nav: [
      { text: 'Guide', link: '/guide/getting-started' },
      { text: 'Capabilities', link: '/capabilities' },
      { text: 'Use cases', link: '/use-cases/' },
      { text: 'Architecture', link: '/architecture/' },
      { text: 'API', link: '/api/rust' },
      { text: 'Roadmap', link: '/project/roadmap' },
    ],
    sidebar: {
      '/guide/': [
        {
          text: 'Guide',
          items: [
            { text: 'Getting started', link: '/guide/getting-started' },
            { text: 'Construction resources', link: '/guide/resources' },
            { text: 'Approvals and constraints', link: '/guide/approvals-constraints' },
            { text: 'Contributing', link: '/guide/contributing' },
          ],
        },
      ],
      '/use-cases/': [
        {
          text: 'Use cases',
          items: [
            { text: 'Overview', link: '/use-cases/' },
            { text: '2D approval plans', link: '/use-cases/2d-approval-plans' },
            { text: 'Structural analysis', link: '/use-cases/structural-analysis' },
          ],
        },
      ],
      '/architecture/': [
        {
          text: 'Architecture',
          items: [
            { text: 'System design', link: '/architecture/' },
            { text: 'Crate map', link: '/architecture/crates' },
            { text: 'The Axiolid boundary', link: '/architecture/axiolid-boundary' },
          ],
        },
        { text: 'Decision records', items: adrItems() },
      ],
      '/adr/': [
        {
          text: 'Decision records',
          items: [{ text: 'Index', link: '/adr/' }].concat(adrItems()),
        },
      ],
      '/api/': [
        {
          text: 'API reference',
          items: [{ text: 'Rust', link: '/api/rust' }],
        },
      ],
      '/project/': [
        {
          text: 'Project',
          items: [
            { text: 'Roadmap', link: '/project/roadmap' },
            { text: 'Changelog', link: '/project/changelog' },
          ],
        },
      ],
      '/': [
        {
          text: 'Start here',
          items: [
            { text: 'Overview', link: '/' },
            { text: 'Getting started', link: '/guide/getting-started' },
            { text: 'Capabilities', link: '/capabilities' },
            { text: 'Use cases', link: '/use-cases/' },
          ],
        },
        {
          text: 'Architecture',
          items: [
            { text: 'System design', link: '/architecture/' },
            { text: 'Crate map', link: '/architecture/crates' },
            { text: 'The Axiolid boundary', link: '/architecture/axiolid-boundary' },
          ],
        },
        {
          text: 'Project',
          items: [
            { text: 'Roadmap', link: '/project/roadmap' },
            { text: 'Changelog', link: '/project/changelog' },
            { text: 'Decision records', link: '/adr/' },
            { text: 'Contributing', link: '/guide/contributing' },
          ],
        },
      ],
    },
    socialLinks: [{ icon: 'github', link: 'https://github.com/openbimrs/ifc' }],
    editLink: {
      pattern: 'https://github.com/openbimrs/ifc/edit/main/docs/:path',
      text: 'Edit this page on GitHub',
    },
    search: { provider: 'local' },
    footer: {
      message: 'Released under the AGPL-3.0-or-later licence. ISO and CEN standards material is not redistributed.',
      copyright: 'Copyright (c) 2026 openbimrs contributors',
    },
  },
})
