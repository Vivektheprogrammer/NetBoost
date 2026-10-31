/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  darkMode: 'class',
  theme: {
    extend: {
      colors: {
        dark: {
          900: '#0a0d14',
          800: '#111726',
          700: '#1a2236',
          600: '#26314d',
        },
        accent: {
          cyan: '#00e5ff',
          blue: '#2979ff',
          emerald: '#00e676',
          amber: '#ffab00',
          rose: '#ff1744',
        }
      },
      fontFamily: {
        sans: ['Inter', 'system-ui', '-apple-system', 'Segoe UI', 'Roboto', 'sans-serif'],
        mono: ['JetBrains Mono', 'Fira Code', 'Consolas', 'monospace'],
      }
    },
  },
  plugins: [],
}

