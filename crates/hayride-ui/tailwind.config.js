/** @type {import('tailwindcss').Config} */
module.exports = {
    content: ["*.html", "./src/**/*.rs","node_modules/preline/dist/*.js"],
    theme: {
      fontFamily: {
        sans: ['Inter', 'sans-serif'],
      },
      extend: {
      },
    },
    plugins: [require("preline/plugin")],
  }