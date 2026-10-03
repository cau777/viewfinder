import { createTheme, type MantineColorsTuple } from '@mantine/core'

// Neon violet -> cyan palette. Change these two tuples to re-skin the whole app.
// Generate new ones at https://mantine.dev/colors-generator/
const neon: MantineColorsTuple = [
  '#f3ecff', '#e1d4ff', '#c0a6ff', '#9d74ff', '#8049fe',
  '#6d2dfe', '#6420ff', '#5313e4', '#490ecc', '#3d06b4',
]
const cyber: MantineColorsTuple = [
  '#dffdff', '#caf6ff', '#99ecff', '#64e2ff', '#3cd9fe',
  '#23d3fe', '#08d0ff', '#00b8e4', '#00a4cc', '#008eb4',
]

export const theme = createTheme({
  primaryColor: 'neon',
  primaryShade: 5,
  colors: { neon, cyber },
  defaultGradient: { from: 'neon.5', to: 'cyber.5', deg: 120 },
  defaultRadius: 'lg',
  fontFamily: '"Space Grotesk", system-ui, sans-serif',
  fontFamilyMonospace: '"JetBrains Mono", ui-monospace, monospace',
  headings: { fontFamily: '"Space Grotesk", system-ui, sans-serif', fontWeight: '700' },
  components: {
    // Every Paper is a frosted-glass panel (see .glass in app.css).
    Paper: { defaultProps: { className: 'glass', p: 'lg' } },
    Button: { defaultProps: { variant: 'gradient', className: 'glow' } },
    Badge: { defaultProps: { variant: 'gradient' } },
  },
})
