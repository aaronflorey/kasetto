import type { Config } from "tailwindcss";

const config: Config = {
  content: ["./app/**/*.{ts,tsx}"],
  theme: {
    extend: {
      fontFamily: {
        mono: ["JetBrains Mono", "Menlo", "Consolas", "monospace"],
        sans: ["DM Sans", "system-ui", "sans-serif"],
      },
    },
  },
};

export default config;
