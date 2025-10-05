/** @type {import('tailwindcss').Config} */
const { addDynamicIconSelectors } = require('@iconify/tailwind');

module.exports = {
  content: [
    "./index.html",
    "./src/**/*.rs",
  ],
  theme: {
    extend: {},
  },
  plugins: [
    require('daisyui'),
    addDynamicIconSelectors(),
  ],
  daisyui: {
    themes: [
      "business", // Light theme
      {
        dark: {
          primary: "#569CD6",
          secondary: "#C586C0",
          accent: "#4EC9B0",
          neutral: "#1e1e1e",
          "base-100": "#1e1e1e",
          "base-200": "#252526",
          "base-300": "#2d2d2d",
          "base-content": "#d4d4d4",
          info: "#3ABFF8",
          success: "#36D399",
          warning: "#FBBD23",
          error: "#F87272",
        },
      },
    ],
    darkTheme: "dark",
  },
}
