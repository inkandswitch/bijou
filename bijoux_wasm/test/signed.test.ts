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
