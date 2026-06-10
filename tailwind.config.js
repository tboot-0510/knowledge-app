/** @type {import('tailwindcss').Config} */
export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: {
    extend: {
      colors: {
        // Warm "paper" light theme inspired by minimal vocabulary apps.
        paper: "#f5f4ef",
        card: "#ffffff",
        ink: "#1b1a18",
        accent: "#4f46e5",
      },
      fontFamily: {
        display: ['"Iowan Old Style"', "Georgia", "Cambria", "serif"],
      },
    },
  },
  plugins: [],
};
