// Mantine's PostCSS preset: enables light-dark(), rem(), and the $mantine-breakpoint-* vars
// in your own .module.css files. https://mantine.dev/styles/postcss-preset/
module.exports = {
  plugins: {
    'postcss-preset-mantine': {},
    'postcss-simple-vars': {
      variables: {
        'mantine-breakpoint-xs': '36em',
        'mantine-breakpoint-sm': '48em',
        'mantine-breakpoint-md': '62em',
        'mantine-breakpoint-lg': '75em',
        'mantine-breakpoint-xl': '88em',
      },
    },
  },
}
