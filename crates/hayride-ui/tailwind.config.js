/** @type {import('tailwindcss').Config} */
module.exports = {
    content: ["*.html", "./src/**/*.rs",],
    daisyui: {
      themes: ["retro",
        {
          custom: {
          "primary": "#1e4b97",
                    
          "secondary": "#302d2b",
                    
          "accent": "#0c72ee",
                    
          "neutral": "#Fbfbfb",
                    
          "base-100": "#EDF1f7",
                    
          "info": "#2ec1ca",
                    
          "success": "#0c72ee",
                    
          "warning": "#F6BE49",
                    
          "error": "#BE202E",
            },
          },
      ],
    },
    theme: {
      fontFamily: {
        sans: ['Inter', 'sans-serif'],
        shrikhand: ['"Shrikhand"', 'cursive'],
      },
      extend: {
      },
    },
    plugins: [require('daisyui')],
  }