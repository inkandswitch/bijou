import { expect } from "chai";
import * as bijou128 from "../dist/esm/node.js";

/**
 * Node.js smoke tests for the wasm-bodge-built `bijou128` npm package.
 *
 * These tests are the Node counterpart to `e2e/bijou128.spec.ts` (which
 * runs the same surface in real browsers via Playwright). Together they
 * exercise both `package.json` `exports` paths: the `node` condition
 * (this file) and the `browser` condition (the Playwright spec).
 *
 * A regression that appears in one runner but not the other almost
 * certainly indicates a wasm-bodge entrypoint divergence — for example,
 * the Node entrypoint loading wasm via `fs.readFileSync` vs the web
 * entrypoint loading it from a base64-embedded blob.
 */

const U128_MAX = (1n << 128n) - 1n;

describe("bijou128 (node)", () => {
  describe("constants", () => {
    it("MAX_BYTES_U128 is 17", () => {
      expect(bijou128.MAX_BYTES_U128()).to.equal(17);
    });
  });

  describe("encodeU128 (happy path)", () => {
    it("encodes tier-0 values as a single byte equal to the value", () => {
      expect([...bijou128.encodeU128(0n)]).to.deep.equal([0x00]);
      expect([...bijou128.encodeU128(42n)]).to.deep.equal([0x2a]);
      expect([...bijou128.encodeU128(239n)]).to.deep.equal([0xef]);
    });

    it("encodes tier-1 values with offset", () => {
      expect([...bijou128.encodeU128(240n)]).to.deep.equal([0xf0, 0x00]);
      expect([...bijou128.encodeU128(300n)]).to.deep.equal([0xf0, 0x3c]);
      expect([...bijou128.encodeU128(495n)]).to.deep.equal([0xf0, 0xff]);
    });

    it("encodes u128::MAX as 17 bytes starting with 0xFF", () => {
      const bytes = bijou128.encodeU128(U128_MAX);
      expect(bytes.length).to.equal(17);
      expect(bytes[0]).to.equal(0xff);
    });

    it("accepts the boundary values 0n and u128::MAX exactly", () => {
      expect([...bijou128.encodeU128(0n)]).to.deep.equal([0x00]);
      expect([...bijou128.encodeU128(U128_MAX)]).to.deep.equal([
        0xff, 0xfe, 0xfe, 0xfe, 0xfe, 0xfe, 0xfe, 0xfe,
        0xfe, 0xfe, 0xfe, 0xfe, 0xfe, 0xfe, 0xfe, 0xfe,
        0x0f,
      ]);
    });
  });

  describe("encodedLenU128", () => {
    it("agrees with encodeU128().length across tier boundaries", () => {
      const cases: bigint[] = [
        0n, 239n, 240n, 495n, 496n, 65_535n, 66_031n, 66_032n,
        1n << 32n, 1n << 64n, 1n << 96n, U128_MAX - 1n, U128_MAX,
      ];
      for (const v of cases) {
        expect(bijou128.encodedLenU128(v), `encodedLenU128(${v}n)`).to.equal(
          bijou128.encodeU128(v).length,
        );
      }
    });
  });

  describe("decodeU128 (happy path)", () => {
    it("round-trips every tier boundary", () => {
      const cases: bigint[] = [
        0n, 1n, 239n, 240n, 495n, 496n, 65_535n, 66_031n, 66_032n,
        1n << 32n, 1n << 64n, 1n << 96n, U128_MAX - 1n, U128_MAX,
      ];
      for (const v of cases) {
        const bytes = bijou128.encodeU128(v);
        const r = bijou128.decodeU128(bytes);
        expect(r.value, `round-trip ${v}n`).to.equal(v);
        expect(r.bytesRead).to.equal(bytes.length);
      }
    });

    it("reports bytesRead < input length when buffer has trailing data", () => {
      const head = bijou128.encodeU128(500n); // 3 bytes
      const buf = new Uint8Array(head.length + 3);
      buf.set(head, 0);
      buf.set([0xaa, 0xbb, 0xcc], head.length);
      const r = bijou128.decodeU128(buf);
      expect(r.value).to.equal(500n);
      expect(r.bytesRead).to.equal(3);
      expect(buf.length).to.equal(6);
    });
  });

  describe("decodeU128 (errors)", () => {
    it("throws Bijou128DecodeError on empty input", () => {
      try {
        bijou128.decodeU128(new Uint8Array([]));
        expect.fail("decodeU128 did not throw");
      } catch (e: any) {
        expect(e.name).to.equal("Bijou128DecodeError");
        expect(e.message).to.contain("buffer too short");
      }
    });

    it("throws Bijou128DecodeError on truncated tier-16 input", () => {
      try {
        // 0xFF needs 16 payload bytes; supply 15.
        bijou128.decodeU128(
          new Uint8Array([0xff, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
        );
        expect.fail("decodeU128 did not throw");
      } catch (e: any) {
        expect(e.name).to.equal("Bijou128DecodeError");
      }
    });

    it("Bijou128DecodeError thrown is an instance of the platform Error", () => {
      try {
        bijou128.decodeU128(new Uint8Array([0xf0]));
        expect.fail("decodeU128 did not throw");
      } catch (e: any) {
        expect(e).to.be.an.instanceOf(Error);
        expect(e.stack).to.be.a("string").and.not.empty;
      }
    });

    it("throws TypeError on non-Uint8Array input (no silent truncation)", () => {
      // Guards the shipped dist against the silent-truncation footgun:
      // a plain JS Array would be coerced via `new Uint8Array(arr)`,
      // bitwise-truncating out-of-range elements (1000 & 0xFF === 232).
      // decodeU128/decodeAllU128 must reject anything that isn't a real
      // Uint8Array. Exercised here through the published package (the
      // wasm-bindgen ABI layer is covered separately in tests/wasm.rs).
      const bad: unknown[] = [[1000], [0x00], null, 42, "nope"];
      for (const input of bad) {
        expect(() => (bijou128.decodeU128 as any)(input)).to.throw(TypeError);
        expect(() => (bijou128.decodeAllU128 as any)(input)).to.throw(TypeError);
      }
    });
  });

  describe("encodeU128 (errors)", () => {
    it("throws RangeError for bigint >= 2n ** 128n", () => {
      // wasm-bindgen's default bigint → u128 marshalling silently
      // truncates via BigInt.asUintN(128, value). bijou's API instead
      // rejects with a RangeError so the caller cannot accidentally
      // violate canonicality at the boundary.
      try {
        bijou128.encodeU128(1n << 128n);
        expect.fail("encodeU128 did not throw");
      } catch (e: any) {
        expect(e).to.be.an.instanceOf(RangeError);
        expect(e.message).to.contain("2**128");
      }
    });

    it("throws RangeError for negative bigint", () => {
      // Without the range check, two's-complement wraparound would
      // encodeU128 `-1n` as the bytes for u128::MAX — a silent footgun.
      const cases: (() => unknown)[] = [
        () => bijou128.encodeU128(-1n),
        () => bijou128.encodeU128(-(1n << 127n)),
        () => bijou128.encodedLenU128(-1n),
      ];
      for (const fn of cases) {
        expect(fn).to.throw(RangeError);
      }
    });

    it("throws TypeError for non-bigint inputs", () => {
      const cases: (() => unknown)[] = [
        () => (bijou128.encodeU128 as any)(42),
        () => (bijou128.encodeU128 as any)("300"),
        () => (bijou128.encodeU128 as any)(null),
        () => (bijou128.encodeU128 as any)(undefined),
        () => (bijou128.encodedLenU128 as any)(42),
        () => (bijou128.encodedLenU128 as any)({}),
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

  describe("decodeAllU128", () => {
    it("returns an Array of bigints with every value in the buffer", () => {
      // Unlike bijou64, there is no BigUint128Array in the web
      // platform, so decodeAllU128 returns a plain Array<bigint>.
      const merged = new Uint8Array([
        ...bijou128.encodeU128(42n),
        ...bijou128.encodeU128(500n),
        ...bijou128.encodeU128(65_535n),
        ...bijou128.encodeU128(1n << 64n),
      ]);
      const values = bijou128.decodeAllU128(merged);
      expect(values).to.be.an("array");
      expect(values.length).to.equal(4);
      expect(values).to.deep.equal([42n, 500n, 65_535n, 1n << 64n]);
    });

    it("returns an empty Array on an empty buffer", () => {
      const empty = bijou128.decodeAllU128(new Uint8Array(0));
      expect(empty).to.be.an("array");
      expect(empty.length).to.equal(0);
    });

    it("throws Bijou128DecodeError on a malformed element", () => {
      try {
        bijou128.decodeAllU128(new Uint8Array([0x42, 0xf0]));
        expect.fail("decodeAllU128 did not throw");
      } catch (e: any) {
        expect(e.name).to.equal("Bijou128DecodeError");
        expect(e).to.be.an.instanceOf(Error);
      }
    });
  });
});
