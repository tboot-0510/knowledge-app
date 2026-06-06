/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: {
    extend: {
      colors: {
        ink: "#0b0f17",
        panel: "#121826",
        accent: "#6ea8fe",
      },
    },
  },
  plugins: [],
};
