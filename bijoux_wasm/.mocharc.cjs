// Mocha configuration for the Node.js JS-package test layer.
//
// Tests in `test/` exercise the wasm-bodge-built `dist/` output the same
// way a Node consumer would (`import { encode, decode } from "@inkandswitch/bijoux"`).
// This complements `test:js:browser` (Playwright) — together they cover
// the full set of `package.json` `exports` entry points.
//
// `tsx` is used as a Node loader so test files can be authored in
// TypeScript without a separate compile step.

module.exports = {
  extension: ["ts"],
  spec: ["test/**/*.test.ts"],
  // `tsx 4.x` uses Node's `--import` hooks API; the legacy `--loader`
  // flag was deprecated in Node 20.6 / 18.19.
  "node-option": ["import=tsx/esm"],
  timeout: 10_000,
  reporter: "spec",
};
