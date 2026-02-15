import js from "@eslint/js"
import tseslint from "typescript-eslint"
import react from "eslint-plugin-react"
import reactHooks from "eslint-plugin-react-hooks"
import reactCompiler from "eslint-plugin-react-compiler"
import simpleImportSort from "eslint-plugin-simple-import-sort"
import boundaries from "eslint-plugin-boundaries"

export default tseslint.config(
  js.configs.recommended,
  ...tseslint.configs.recommended,
  {
    files: ["src/**/*.{ts,tsx}"],
    plugins: {
      react,
      "react-hooks": reactHooks,
      "react-compiler": reactCompiler,
      "simple-import-sort": simpleImportSort,
      boundaries,
    },
    languageOptions: {
      parserOptions: {
        ecmaFeatures: {
          jsx: true,
        },
      },
    },
    settings: {
      react: {
        version: "detect",
      },
      "import/resolver": {
        typescript: {
          project: "./tsconfig.json",
        },
      },
      "boundaries/elements": [
        { type: "routes", pattern: "src/routes/*", capture: ["route"] },
        { type: "ui", pattern: "src/components/ui/*", mode: "file" },
        { type: "components", pattern: "src/components/*", mode: "file" },
        {
          type: "component-modules",
          pattern: "src/components/*",
          mode: "folder",
        },
        { type: "hooks", pattern: "src/hooks/*" },
        { type: "context", pattern: "src/context/*" },
        { type: "lib", pattern: "src/lib/*" },
        { type: "bindings", pattern: "src/bindings.ts", mode: "file" },
        { type: "app", pattern: "src/*", mode: "file" },
      ],
      "boundaries/include": ["src/**/*.{ts,tsx}"],
    },
    rules: {
      ...react.configs.recommended.rules,
      ...react.configs["jsx-runtime"].rules,
      ...reactHooks.configs.recommended.rules,
      "react-compiler/react-compiler": "error",
      "react/prop-types": "off", // Not needed with TypeScript
      "simple-import-sort/imports": "error",
      "simple-import-sort/exports": "error",
      "boundaries/element-types": [
        "error",
        {
          default: "disallow",
          rules: [
            {
              from: "routes",
              allow: [
                "lib",
                "ui",
                "components",
                "component-modules",
                "hooks",
                "context",
                "bindings",
                ["routes", { route: "${from.route}" }],
              ],
            },
            {
              from: ["components", "component-modules"],
              allow: [
                "lib",
                "ui",
                "components",
                "component-modules",
                "hooks",
                "context",
                "bindings",
              ],
            },
            {
              from: "ui",
              allow: ["ui"],
            },
            {
              from: "hooks",
              allow: ["lib", "hooks", "context", "bindings"],
            },
            {
              from: "context",
              allow: ["lib", "hooks", "context", "bindings"],
            },
            { from: "lib", allow: ["lib", "bindings"] },
            {
              from: "app",
              allow: [
                "ui",
                "components",
                "component-modules",
                "context",
                "app",
              ],
            },
          ],
        },
      ],
      "boundaries/entry-point": [
        "error",
        {
          default: "disallow",
          rules: [
            {
              target: "component-modules",
              allow: ["index.ts", "index.tsx"],
            },
            {
              target: [
                "routes",
                "ui",
                "components",
                "hooks",
                "context",
                "lib",
                "bindings",
                "app",
              ],
              allow: "*",
            },
          ],
        },
      ],
    },
  },
  {
    ignores: ["src/bindings.ts", "src-tauri/**", "dist/**"],
  },
)
