import { dirname } from "node:path";
import { fileURLToPath } from "node:url";

import { FlatCompat } from "@eslint/eslintrc";
import importPlugin from "eslint-plugin-import";

const directory = dirname(fileURLToPath(import.meta.url));
const compat = new FlatCompat({ baseDirectory: directory });
const eslintConfig = [
  {
    ignores: [".next/**", "node_modules/**", "out/**", "dist/**", "coverage/**"],
  },
  ...compat.extends("next/core-web-vitals", "prettier"),
  {
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
          groups: [
            "builtin",
            "external",
            "internal",
            "parent",
            "sibling",
            "index",
            "object",
            "type",
          ],
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

export default eslintConfig;
