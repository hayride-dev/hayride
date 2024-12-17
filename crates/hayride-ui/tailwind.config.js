/** @type {import('tailwindcss').Config} */
module.exports = {
    content: ["*.html", "./src/**/*.rs",],
    daisyui: {
      themes: ["retro"],
    },
    theme: {
      fontFamily: {
        sans: ['Inter', 'sans-serif'],
      },
      extend: {
      },
    },
    plugins: [require('daisyui')],
  }