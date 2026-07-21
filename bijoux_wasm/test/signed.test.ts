import { expect } from "chai";
import * as bijoux from "../dist/esm/node.js";

/**
 * Node.js smoke tests for the signed exports (bijou32s / bijou64s /
 * bijou128s) of the wasm-bodge-built `@inkandswitch/bijoux` package.
 *
 * Signed carriers mirror the unsigned ones: i32 uses `number`
 * (`decodeAllI32` → `Int32Array`), i64 and i128 use `bigint`
 * (`BigInt64Array` and `Array<bigint>` respectively). Negative values
 * are the interesting cases here — zigzag makes small negatives
 * single-byte, and the range checks are two-sided.
 */

describe("bijou32s (node)", () => {
  it("MAX_BYTES_I32 is 5", () => {
    expect(bijoux.MAX_BYTES_I32()).to.equal(5);
  });

  it("encodes small values of both signs as single bytes", () => {
    expect([...bijoux.encodeI32(0)]).to.deep.equal([0x00]);
    expect([...bijoux.encodeI32(-1)]).to.deep.equal([0x01]);
    expect([...bijoux.encodeI32(1)]).to.deep.equal([0x02]);
    expect([...bijoux.encodeI32(-126)]).to.deep.equal([0xfb]);
    expect([...bijoux.encodeI32(125)]).to.deep.equal([0xfa]);
  });

  it("round-trips the extremes", () => {
    for (const v of [-2147483648, 2147483647]) {
      const { value, bytesRead } = bijoux.decodeI32(bijoux.encodeI32(v));
      expect(value).to.equal(v);
      expect(bytesRead).to.equal(5);
    }
  });

  it("decodeAllI32 returns an Int32Array", () => {
    const buf = new Uint8Array([
      ...bijoux.encodeI32(-2),
      ...bijoux.encodeI32(0),
      ...bijoux.encodeI32(2),
    ]);
    const values = bijoux.decodeAllI32(buf);
    expect(values).to.be.instanceOf(Int32Array);
    expect([...values]).to.deep.equal([-2, 0, 2]);
  });

  it("throws RangeError outside [-(2**31), 2**31)", () => {
    expect(() => bijoux.encodeI32(2 ** 31)).to.throw(RangeError);
    expect(() => bijoux.encodeI32(-(2 ** 31) - 1)).to.throw(RangeError);
  });

  it("throws TypeError for non-number input", () => {
    expect(() => bijoux.encodeI32(1n as any)).to.throw(TypeError);
  });

  it("decode errors carry name Bijou32sDecodeError", () => {
    try {
      bijoux.decodeI32(new Uint8Array([0xfc]));
      expect.fail("should have thrown");
    } catch (e: any) {
      expect(e.name).to.equal("Bijou32sDecodeError");
    }
  });
});

describe("bijou64s (node)", () => {
  it("MAX_BYTES_I64 is 9", () => {
    expect(bijoux.MAX_BYTES_I64()).to.equal(9);
  });

  it("encodes small values of both signs as single bytes", () => {
    expect([...bijoux.encodeI64(0n)]).to.deep.equal([0x00]);
    expect([...bijoux.encodeI64(-1n)]).to.deep.equal([0x01]);
    expect([...bijoux.encodeI64(1n)]).to.deep.equal([0x02]);
    expect([...bijoux.encodeI64(-124n)]).to.deep.equal([0xf7]);
    expect([...bijoux.encodeI64(123n)]).to.deep.equal([0xf6]);
  });

  it("round-trips the extremes", () => {
    for (const v of [-(2n ** 63n), 2n ** 63n - 1n]) {
      const { value, bytesRead } = bijoux.decodeI64(bijoux.encodeI64(v));
      expect(value).to.equal(v);
      expect(bytesRead).to.equal(9);
    }
  });

  it("decodeAllI64 returns a BigInt64Array", () => {
    const buf = new Uint8Array([
      ...bijoux.encodeI64(-300n),
      ...bijoux.encodeI64(0n),
      ...bijoux.encodeI64(300n),
    ]);
    const values = bijoux.decodeAllI64(buf);
    expect(values).to.be.instanceOf(BigInt64Array);
    expect([...values]).to.deep.equal([-300n, 0n, 300n]);
  });

  it("throws RangeError outside [-(2n**63n), 2n**63n)", () => {
    expect(() => bijoux.encodeI64(2n ** 63n)).to.throw(RangeError);
    expect(() => bijoux.encodeI64(-(2n ** 63n) - 1n)).to.throw(RangeError);
  });

  it("throws TypeError for non-bigint input", () => {
    expect(() => bijoux.encodeI64(42 as any)).to.throw(TypeError);
  });

  it("decode errors carry name Bijou64sDecodeError", () => {
    try {
      bijoux.decodeI64(new Uint8Array([0xf8]));
      expect.fail("should have thrown");
    } catch (e: any) {
      expect(e.name).to.equal("Bijou64sDecodeError");
    }
  });
});

describe("bijou128s (node)", () => {
  it("MAX_BYTES_I128 is 17", () => {
    expect(bijoux.MAX_BYTES_I128()).to.equal(17);
  });

  it("encodes small values of both signs as single bytes", () => {
    expect([...bijoux.encodeI128(0n)]).to.deep.equal([0x00]);
    expect([...bijoux.encodeI128(-1n)]).to.deep.equal([0x01]);
    expect([...bijoux.encodeI128(-120n)]).to.deep.equal([0xef]);
    expect([...bijoux.encodeI128(119n)]).to.deep.equal([0xee]);
  });

  it("round-trips the extremes", () => {
    for (const v of [-(2n ** 127n), 2n ** 127n - 1n]) {
      const { value, bytesRead } = bijoux.decodeI128(bijoux.encodeI128(v));
      expect(value).to.equal(v);
      expect(bytesRead).to.equal(17);
    }
  });

  it("decodeAllI128 returns a plain Array of bigints", () => {
    const buf = new Uint8Array([
      ...bijoux.encodeI128(-(2n ** 100n)),
      ...bijoux.encodeI128(2n ** 100n),
    ]);
    const values = bijoux.decodeAllI128(buf);
    expect(Array.isArray(values)).to.equal(true);
    expect(values).to.deep.equal([-(2n ** 100n), 2n ** 100n]);
  });

  it("throws RangeError outside [-(2n**127n), 2n**127n)", () => {
    expect(() => bijoux.encodeI128(2n ** 127n)).to.throw(RangeError);
    expect(() => bijoux.encodeI128(-(2n ** 127n) - 1n)).to.throw(RangeError);
  });

  it("decode errors carry name Bijou128sDecodeError", () => {
    try {
      bijoux.decodeI128(new Uint8Array([0xf0]));
      expect.fail("should have thrown");
    } catch (e: any) {
      expect(e.name).to.equal("Bijou128sDecodeError");
    }
  });
});

describe("signed carrier edge cases (node)", () => {
  it("encodeI32(-0) folds to 0", () => {
    expect([...bijoux.encodeI32(-0)]).to.deep.equal([0x00]);
  });

  it("encodeI32 rejects fractional negatives with TypeError", () => {
    expect(() => bijoux.encodeI32(-1.5)).to.throw(TypeError);
  });

  it("encodeI32 rejects NaN and ±Infinity with TypeError", () => {
    for (const bad of [NaN, Infinity, -Infinity]) {
      expect(() => bijoux.encodeI32(bad)).to.throw(TypeError);
    }
  });

  it("encodeI32 rejects MIN/MAX_SAFE_INTEGER (outside i32) with RangeError", () => {
    expect(() => bijoux.encodeI32(Number.MIN_SAFE_INTEGER)).to.throw(RangeError);
    expect(() => bijoux.encodeI32(Number.MAX_SAFE_INTEGER)).to.throw(RangeError);
  });

  it("encodeI32 bounds are exact at ±2**31", () => {
    expect(bijoux.decodeI32(bijoux.encodeI32(-(2 ** 31))).value).to.equal(-(2 ** 31));
    expect(bijoux.decodeI32(bijoux.encodeI32(2 ** 31 - 1)).value).to.equal(2 ** 31 - 1);
    expect(() => bijoux.encodeI32(2 ** 31)).to.throw(RangeError);
    expect(() => bijoux.encodeI32(-(2 ** 31) - 1)).to.throw(RangeError);
  });

  it("encodeI128 rejects a number carrier with TypeError", () => {
    expect(() => bijoux.encodeI128(42 as any)).to.throw(TypeError);
  });

  it("encodedLenI* agrees with encode().length at the signed tier edges", () => {
    for (const v of [0, -1, 1, 125, -126, 126, -127, 253, -254, 254]) {
      expect(bijoux.encodedLenI32(v)).to.equal(bijoux.encodeI32(v).length);
    }
    for (const v of [0n, -1n, 123n, -124n, 124n, -125n, 251n, -252n, 252n]) {
      expect(bijoux.encodedLenI64(v)).to.equal(bijoux.encodeI64(v).length);
    }
    for (const v of [0n, -1n, 119n, -120n, 120n, -121n, 247n, -248n, 248n]) {
      expect(bijoux.encodedLenI128(v)).to.equal(bijoux.encodeI128(v).length);
    }
  });

  it("decodeAllI* on an empty buffer returns an empty result", () => {
    expect(bijoux.decodeAllI32(new Uint8Array()).length).to.equal(0);
    expect(bijoux.decodeAllI64(new Uint8Array()).length).to.equal(0);
    expect(bijoux.decodeAllI128(new Uint8Array()).length).to.equal(0);
  });

  it("decodeAllI* throws on a malformed tail (all-or-nothing)", () => {
    const buf = new Uint8Array([...bijoux.encodeI64(-1n), 0xf8]); // truncated tail
    expect(() => bijoux.decodeAllI64(buf)).to.throw();
  });

  it("overflow decodes carry the signed error names", () => {
    const cases: [Uint8Array, string, (b: Uint8Array) => unknown][] = [
      [new Uint8Array(5).fill(0xff), "Bijou32sDecodeError", (b) => bijoux.decodeI32(b)],
      [new Uint8Array(9).fill(0xff), "Bijou64sDecodeError", (b) => bijoux.decodeI64(b)],
      [new Uint8Array(17).fill(0xff), "Bijou128sDecodeError", (b) => bijoux.decodeI128(b)],
    ];
    for (const [buf, name, decode] of cases) {
      try {
        decode(buf);
        expect.fail("should have thrown");
      } catch (e: any) {
        expect(e.name).to.equal(name);
      }
    }
  });

  it("decode leaves trailing bytes untouched (bytesRead < length)", () => {
    const buf = new Uint8Array([0x01, 0xaa, 0xbb]);
    const { value, bytesRead } = bijoux.decodeI64(buf);
    expect(value).to.equal(-1n);
    expect(bytesRead).to.equal(1);
  });

  it("round-trips BigInt(MIN_SAFE_INTEGER) through i64", () => {
    const v = BigInt(Number.MIN_SAFE_INTEGER);
    expect(bijoux.decodeI64(bijoux.encodeI64(v)).value).to.equal(v);
  });
});
