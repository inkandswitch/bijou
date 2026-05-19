import { expect } from "chai";
import * as bijou64 from "../dist/esm/node.js";

/**
 * Node.js smoke tests for the wasm-bodge-built `bijou64` npm package.
 *
 * These tests are the Node counterpart to `e2e/bijou64.spec.ts` (which
 * runs the same surface in real browsers via Playwright). Together they
 * exercise both `package.json` `exports` paths: the `node` condition
 * (this file) and the `browser` condition (the Playwright spec).
 *
 * A regression that appears in one runner but not the other almost
 * certainly indicates a wasm-bodge entrypoint divergence — for example,
 * the Node entrypoint loading wasm via `fs.readFileSync` vs the web
 * entrypoint loading it from a base64-embedded blob.
 */

describe("bijou64 (node)", () => {
  describe("constants", () => {
    it("MAX_BYTES is 9", () => {
      expect(bijou64.MAX_BYTES()).to.equal(9);
    });
  });

  describe("encode (happy path)", () => {
    it("encodes tier-0 values as a single byte equal to the value", () => {
      expect([...bijou64.encode(0n)]).to.deep.equal([0x00]);
      expect([...bijou64.encode(42n)]).to.deep.equal([0x2a]);
      expect([...bijou64.encode(247n)]).to.deep.equal([0xf7]);
    });

    it("encodes tier-1 values with offset", () => {
      expect([...bijou64.encode(248n)]).to.deep.equal([0xf8, 0x00]);
      expect([...bijou64.encode(300n)]).to.deep.equal([0xf8, 0x34]);
      expect([...bijou64.encode(503n)]).to.deep.equal([0xf8, 0xff]);
    });

    it("encodes u64::MAX as 9 bytes starting with 0xFF", () => {
      const bytes = bijou64.encode((1n << 64n) - 1n);
      expect(bytes.length).to.equal(9);
      expect(bytes[0]).to.equal(0xff);
    });

    it("accepts the boundary values 0n and u64::MAX exactly", () => {
      expect([...bijou64.encode(0n)]).to.deep.equal([0x00]);
      expect([...bijou64.encode((1n << 64n) - 1n)]).to.deep.equal([
        0xff, 0xfe, 0xfe, 0xfe, 0xfe, 0xfe, 0xfe, 0xfe, 0x07,
      ]);
    });
  });

  describe("encodedLen", () => {
    it("agrees with encode().length across tier boundaries", () => {
      const cases: bigint[] = [
        0n, 247n, 248n, 503n, 504n, 65_535n, 66_039n, 66_040n,
        16_843_255n, 1n << 32n, (1n << 64n) - 2n, (1n << 64n) - 1n,
      ];
      for (const v of cases) {
        expect(bijou64.encodedLen(v), `encodedLen(${v}n)`).to.equal(
          bijou64.encode(v).length,
        );
      }
    });
  });

  describe("decode (happy path)", () => {
    it("round-trips every tier boundary", () => {
      const cases: bigint[] = [
        0n, 1n, 247n, 248n, 503n, 504n, 65_535n, 66_039n, 66_040n,
        16_843_255n, 1n << 32n, (1n << 64n) - 2n, (1n << 64n) - 1n,
      ];
      for (const v of cases) {
        const bytes = bijou64.encode(v);
        const r = bijou64.decode(bytes);
        expect(r.value, `round-trip ${v}n`).to.equal(v);
        expect(r.bytesRead).to.equal(bytes.length);
      }
    });

    it("reports bytesRead < input length when buffer has trailing data", () => {
      const head = bijou64.encode(300n); // 2 bytes
      const buf = new Uint8Array(head.length + 3);
      buf.set(head, 0);
      buf.set([0xaa, 0xbb, 0xcc], head.length);
      const r = bijou64.decode(buf);
      expect(r.value).to.equal(300n);
      expect(r.bytesRead).to.equal(2);
      expect(buf.length).to.equal(5);
    });
  });

  describe("decode (errors)", () => {
    it("throws Bijou64DecodeError on empty input", () => {
      try {
        bijou64.decode(new Uint8Array([]));
        expect.fail("decode did not throw");
      } catch (e: any) {
        expect(e.name).to.equal("Bijou64DecodeError");
        expect(e.message).to.contain("buffer too short");
      }
    });

    it("throws Bijou64DecodeError on truncated tier-8 input", () => {
      try {
        // 0xFF needs 8 payload bytes; supply 7.
        bijou64.decode(new Uint8Array([0xff, 0, 0, 0, 0, 0, 0, 0]));
        expect.fail("decode did not throw");
      } catch (e: any) {
        expect(e.name).to.equal("Bijou64DecodeError");
      }
    });

    it("Bijou64DecodeError thrown is an instance of the platform Error", () => {
      // Sanity-check that the JsValue conversion preserves the JS Error
      // prototype chain across the wasm-bindgen boundary.
      try {
        bijou64.decode(new Uint8Array([0xf8]));
        expect.fail("decode did not throw");
      } catch (e: any) {
        expect(e).to.be.an.instanceOf(Error);
        expect(e.stack).to.be.a("string").and.not.empty;
      }
    });
  });

  describe("encode (errors)", () => {
    it("throws RangeError for bigint >= 2n ** 64n", () => {
      // wasm-bindgen's default bigint → u64 marshalling silently
      // truncates via BigInt.asUintN(64, value). bijou's API instead
      // rejects with a RangeError so the caller cannot accidentally
      // violate canonicality at the boundary.
      try {
        bijou64.encode(1n << 64n);
        expect.fail("encode did not throw");
      } catch (e: any) {
        expect(e).to.be.an.instanceOf(RangeError);
        expect(e.message).to.contain("2**64");
      }
    });

    it("throws RangeError for negative bigint", () => {
      // Without the range check, two's-complement wraparound would
      // encode `-1n` as the bytes for u64::MAX — a silent footgun.
      const cases: (() => unknown)[] = [
        () => bijou64.encode(-1n),
        () => bijou64.encode(-(1n << 63n)),
        () => bijou64.encodedLen(-1n),
      ];
      for (const fn of cases) {
        expect(fn).to.throw(RangeError);
      }
    });

    it("throws TypeError for non-bigint inputs", () => {
      // wasm-bindgen does not enforce `&BigInt` at runtime — the JS
      // shim happily accepts any value. We distinguish "wrong type"
      // from "out of range" so callers can give useful diagnostics.
      const cases: (() => unknown)[] = [
        () => (bijou64.encode as any)(42),
        () => (bijou64.encode as any)("300"),
        () => (bijou64.encode as any)(null),
        () => (bijou64.encode as any)(undefined),
        () => (bijou64.encodedLen as any)(42),
        () => (bijou64.encodedLen as any)({}),
      ];
      for (const fn of cases) {
        try {
          fn();
          expect.fail("did not throw");
        } catch (e: any) {
          expect(e).to.be.an.instanceOf(TypeError);
          expect(e.message.toLowerCase()).to.contain("bigint");
        }
      }
    });
  });

  describe("decodeAll", () => {
    it("returns a BigUint64Array of every value in the buffer", () => {
      const merged = new Uint8Array([
        ...bijou64.encode(42n),
        ...bijou64.encode(300n),
        ...bijou64.encode(65_535n),
        ...bijou64.encode(1n << 32n),
      ]);
      const values = bijou64.decodeAll(merged);
      expect(values).to.be.an.instanceOf(BigUint64Array);
      expect(values.length).to.equal(4);
      expect(Array.from(values)).to.deep.equal([
        42n, 300n, 65_535n, 1n << 32n,
      ]);
    });

    it("returns an empty BigUint64Array on an empty buffer", () => {
      const empty = bijou64.decodeAll(new Uint8Array(0));
      expect(empty).to.be.an.instanceOf(BigUint64Array);
      expect(empty.length).to.equal(0);
    });

    it("throws Bijou64DecodeError on a malformed element", () => {
      // [0x42, 0xF8] — first byte decodes successfully, second is a tag
      // with no payload. decodeAll must abort and surface the error,
      // NOT silently return the partial prefix.
      try {
        bijou64.decodeAll(new Uint8Array([0x42, 0xf8]));
        expect.fail("decodeAll did not throw");
      } catch (e: any) {
        expect(e.name).to.equal("Bijou64DecodeError");
        expect(e).to.be.an.instanceOf(Error);
      }
    });
  });
});
