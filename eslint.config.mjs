import eslint from "@eslint/js";
import importPlugin from "eslint-plugin-import";
import tseslint from "typescript-eslint";

export default [
  {
    ignores: ["**/dist/**", "**/node_modules/**"],
  },
  eslint.configs.recommended,
  ...tseslint.configs.recommended,
  {
    files: ["**/*.ts"],
    plugins: {
      import: importPlugin,
    },
    rules: {
      "import/order": [
        "error",
        {
          alphabetize: {
            caseInsensitive: true,
            order: "asc",
          },
          groups: ["builtin", "external", "internal", "parent", "sibling", "index", "type"],
          "newlines-between": "always",
          pathGroupsExcludedImportTypes: ["builtin"],
        },
      ],
      "padding-line-between-statements": [
        "error",
        { blankLine: "always", prev: "function", next: "function" },
        { blankLine: "always", prev: "*", next: "return" },
        { blankLine: "always", prev: "*", next: "throw" },
      ],
    },
  },
];
