import { expect } from "chai";
import * as bijou32 from "../dist/esm/node.js";

/**
 * Node.js smoke tests for the wasm-bodge-built `bijou32` npm package.
 *
 * Counterpart to `e2e/bijou32.spec.ts` (which runs the same surface in
 * real browsers via Playwright). Unlike `bijou64`/`bijou128`, the wasm
 * boundary uses plain JS `number` values, not `bigint` — `u32::MAX`
 * fits inside `Number.MAX_SAFE_INTEGER`.
 */

const U32_MAX = 2 ** 32 - 1;

describe("bijou32 (node)", () => {
  describe("constants", () => {
    it("MAX_BYTES is 5", () => {
      expect(bijou32.MAX_BYTES()).to.equal(5);
    });
  });

  describe("encode (happy path)", () => {
    it("encodes tier-0 values as a single byte equal to the value", () => {
      expect([...bijou32.encode(0)]).to.deep.equal([0x00]);
      expect([...bijou32.encode(42)]).to.deep.equal([0x2a]);
      expect([...bijou32.encode(251)]).to.deep.equal([0xfb]);
    });

    it("encodes tier-1 values with offset", () => {
      expect([...bijou32.encode(252)]).to.deep.equal([0xfc, 0x00]);
      expect([...bijou32.encode(300)]).to.deep.equal([0xfc, 0x30]);
      expect([...bijou32.encode(507)]).to.deep.equal([0xfc, 0xff]);
    });

    it("encodes u32::MAX as 5 bytes starting with 0xFF", () => {
      const bytes = bijou32.encode(U32_MAX);
      expect(bytes.length).to.equal(5);
      expect(bytes[0]).to.equal(0xff);
    });

    it("accepts the boundary values 0 and u32::MAX exactly", () => {
      expect([...bijou32.encode(0)]).to.deep.equal([0x00]);
      expect([...bijou32.encode(U32_MAX)]).to.deep.equal([
        0xff, 0xfe, 0xfe, 0xfe, 0x03,
      ]);
    });
  });

  describe("encodedLen", () => {
    it("agrees with encode().length across tier boundaries", () => {
      const cases: number[] = [
        0, 251, 252, 507, 508, 65_535, 66_043, 66_044,
        16_843_259, 16_843_260, U32_MAX - 1, U32_MAX,
      ];
      for (const v of cases) {
        expect(bijou32.encodedLen(v), `encodedLen(${v})`).to.equal(
          bijou32.encode(v).length,
        );
      }
    });
  });

  describe("decode (happy path)", () => {
    it("round-trips every tier boundary", () => {
      const cases: number[] = [
        0, 1, 251, 252, 507, 508, 65_535, 66_043, 66_044,
        16_843_259, 16_843_260, U32_MAX - 1, U32_MAX,
      ];
      for (const v of cases) {
        const bytes = bijou32.encode(v);
        const r = bijou32.decode(bytes);
        expect(r.value, `round-trip ${v}`).to.equal(v);
        expect(r.bytesRead).to.equal(bytes.length);
      }
    });

    it("reports bytesRead < input length when buffer has trailing data", () => {
      const head = bijou32.encode(300); // 2 bytes
      const buf = new Uint8Array(head.length + 3);
      buf.set(head, 0);
      buf.set([0xaa, 0xbb, 0xcc], head.length);
      const r = bijou32.decode(buf);
      expect(r.value).to.equal(300);
      expect(r.bytesRead).to.equal(2);
      expect(buf.length).to.equal(5);
    });
  });

  describe("decode (errors)", () => {
    it("throws Bijou32DecodeError on empty input", () => {
      try {
        bijou32.decode(new Uint8Array([]));
        expect.fail("decode did not throw");
      } catch (e: any) {
        expect(e.name).to.equal("Bijou32DecodeError");
        expect(e.message).to.contain("buffer too short");
      }
    });

    it("throws Bijou32DecodeError on truncated tier-4 input", () => {
      try {
        // 0xFF needs 4 payload bytes; supply 3.
        bijou32.decode(new Uint8Array([0xff, 0, 0, 0]));
        expect.fail("decode did not throw");
      } catch (e: any) {
        expect(e.name).to.equal("Bijou32DecodeError");
      }
    });

    it("Bijou32DecodeError thrown is an instance of the platform Error", () => {
      try {
        bijou32.decode(new Uint8Array([0xfc]));
        expect.fail("decode did not throw");
      } catch (e: any) {
        expect(e).to.be.an.instanceOf(Error);
        expect(e.stack).to.be.a("string").and.not.empty;
      }
    });

    it("throws TypeError on non-Uint8Array input (no silent truncation)", () => {
      // Guards the shipped dist against the silent-truncation footgun:
      // a plain JS Array would be coerced via `new Uint8Array(arr)`,
      // bitwise-truncating out-of-range elements (1000 & 0xFF === 232).
      // decode/decodeAll must reject anything that isn't a real
      // Uint8Array. Exercised here through the published package (the
      // wasm-bindgen ABI layer is covered separately in tests/wasm.rs).
      const bad: unknown[] = [[1000], [0x00], null, 42, "nope"];
      for (const input of bad) {
        expect(() => (bijou32.decode as any)(input)).to.throw(TypeError);
        expect(() => (bijou32.decodeAll as any)(input)).to.throw(TypeError);
      }
    });
  });

  describe("encode (errors)", () => {
    it("throws RangeError for numbers >= 2**32", () => {
      // Without the range check, JS's `>>> 0` cast silently wraps
      // `2**32` to `0` and `2**32 + 1` to `1` — a silent footgun for
      // content-addressed protocols.
      try {
        bijou32.encode(2 ** 32);
        expect.fail("encode did not throw");
      } catch (e: any) {
        expect(e).to.be.an.instanceOf(RangeError);
        expect(e.message).to.contain("2**32");
      }
    });

    it("throws RangeError for negative numbers", () => {
      // Without the range check, JS's `>>> 0` cast encodes `-1` as
      // u32::MAX. We reject negatives explicitly.
      const cases: (() => unknown)[] = [
        () => bijou32.encode(-1),
        () => bijou32.encode(-(2 ** 31)),
        () => bijou32.encodedLen(-1),
      ];
      for (const fn of cases) {
        expect(fn).to.throw(RangeError);
      }
    });

    it("throws TypeError for fractional numbers", () => {
      // Fractional values are nominally in range but not integers.
      const cases: (() => unknown)[] = [
        () => bijou32.encode(1.5),
        () => bijou32.encode(0.1),
        () => bijou32.encodedLen(3.14),
      ];
      for (const fn of cases) {
        expect(fn).to.throw(TypeError);
      }
    });

    it("throws TypeError for NaN, Infinity, and non-number inputs", () => {
      const cases: (() => unknown)[] = [
        () => bijou32.encode(NaN),
        () => bijou32.encode(Infinity),
        () => bijou32.encode(-Infinity),
        () => (bijou32.encode as any)(42n),         // bigint, not number
        () => (bijou32.encode as any)("300"),       // string
        () => (bijou32.encode as any)(null),
        () => (bijou32.encode as any)(undefined),
        () => (bijou32.encodedLen as any)({}),
      ];
      for (const fn of cases) {
        expect(fn).to.throw(TypeError);
      }
    });
  });

  describe("decodeAll", () => {
    it("returns a Uint32Array of every value in the buffer", () => {
      const merged = new Uint8Array([
        ...bijou32.encode(42),
        ...bijou32.encode(300),
        ...bijou32.encode(65_535),
        ...bijou32.encode(1 << 24),
      ]);
      const values = bijou32.decodeAll(merged);
      expect(values).to.be.an.instanceOf(Uint32Array);
      expect(values.length).to.equal(4);
      expect(Array.from(values)).to.deep.equal([42, 300, 65_535, 1 << 24]);
    });

    it("returns an empty Uint32Array on an empty buffer", () => {
      const empty = bijou32.decodeAll(new Uint8Array(0));
      expect(empty).to.be.an.instanceOf(Uint32Array);
      expect(empty.length).to.equal(0);
    });

    it("throws Bijou32DecodeError on a malformed element", () => {
      try {
        bijou32.decodeAll(new Uint8Array([0x42, 0xfc]));
        expect.fail("decodeAll did not throw");
      } catch (e: any) {
        expect(e.name).to.equal("Bijou32DecodeError");
        expect(e).to.be.an.instanceOf(Error);
      }
    });
  });
});
