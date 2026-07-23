import { test, expect, type Page } from "@playwright/test";

/**
 * Cross-browser smoke tests for the signed exports (bijou32s /
 * bijou64s / bijou128s) of the `@inkandswitch/bijoux` npm package.
 *
 * Like the unsigned specs, everything goes through `window.bijoux`
 * (the wasm-bodge "web" entrypoint) and BigInt-producing expressions
 * stay inside `page.evaluate` so Playwright never has to serialise
 * BigInts across its IPC boundary. Negative values are the focus:
 * zigzag single-byte negatives, two-sided range checks, and signed
 * typed-array returns.
 */

const SERVER_URL = "/e2e/server/index.html";

test.beforeEach(async ({ page }: { page: Page }) => {
  await page.goto(SERVER_URL);
  await page.waitForFunction(() => (window as any).bijouxReady === true, {
    timeout: 30_000,
  });
});

test("signed MAX_BYTES are 5 / 9 / 17", async ({ page }) => {
  const result = await page.evaluate(() => {
    const b = (window as any).bijoux;
    return [b.MAX_BYTES_I32(), b.MAX_BYTES_I64(), b.MAX_BYTES_I128()];
  });
  expect(result).toEqual([5, 9, 17]);
});

test("small negatives encode as single bytes in every width", async ({ page }) => {
  const result = await page.evaluate(() => {
    const b = (window as any).bijoux;
    return {
      i32: [...b.encodeI32(-1)],
      i64: [...b.encodeI64(-1n)],
      i128: [...b.encodeI128(-1n)],
    };
  });
  expect(result).toEqual({ i32: [0x01], i64: [0x01], i128: [0x01] });
});

test("i64 round-trips the extremes through decode", async ({ page }) => {
  const result = await page.evaluate(() => {
    const b = (window as any).bijoux;
    const min = b.decodeI64(b.encodeI64(-(2n ** 63n)));
    const max = b.decodeI64(b.encodeI64(2n ** 63n - 1n));
    return {
      minOk: min.value === -(2n ** 63n) && min.bytesRead === 9,
      maxOk: max.value === 2n ** 63n - 1n && max.bytesRead === 9,
    };
  });
  expect(result).toEqual({ minOk: true, maxOk: true });
});

test("decodeAllI32 returns Int32Array; decodeAllI64 returns BigInt64Array", async ({ page }) => {
  const result = await page.evaluate(() => {
    const b = (window as any).bijoux;
    const buf32 = new Uint8Array([...b.encodeI32(-2), ...b.encodeI32(2)]);
    const buf64 = new Uint8Array([...b.encodeI64(-300n), ...b.encodeI64(300n)]);
    const out32 = b.decodeAllI32(buf32);
    const out64 = b.decodeAllI64(buf64);
    return {
      is32: out32 instanceof Int32Array,
      vals32: [...out32],
      is64: out64 instanceof BigInt64Array,
      ok64: out64[0] === -300n && out64[1] === 300n,
    };
  });
  expect(result).toEqual({ is32: true, vals32: [-2, 2], is64: true, ok64: true });
});

test("two-sided range errors throw RangeError", async ({ page }) => {
  const result = await page.evaluate(() => {
    const b = (window as any).bijoux;
    const throws = (fn: () => void) => {
      try {
        fn();
        return "no-throw";
      } catch (e: any) {
        return e.constructor.name;
      }
    };
    return [
      throws(() => b.encodeI32(2 ** 31)),
      throws(() => b.encodeI32(-(2 ** 31) - 1)),
      throws(() => b.encodeI64(2n ** 63n)),
      throws(() => b.encodeI64(-(2n ** 63n) - 1n)),
      throws(() => b.encodeI128(2n ** 127n)),
    ];
  });
  expect(result).toEqual(Array(5).fill("RangeError"));
});

test("decode errors carry width-specific *sDecodeError names", async ({ page }) => {
  const result = await page.evaluate(() => {
    const b = (window as any).bijoux;
    const nameOf = (fn: () => void) => {
      try {
        fn();
        return "no-throw";
      } catch (e: any) {
        return e.name;
      }
    };
    return [
      nameOf(() => b.decodeI32(new Uint8Array([0xfc]))),
      nameOf(() => b.decodeI64(new Uint8Array([0xf8]))),
      nameOf(() => b.decodeI128(new Uint8Array([0xf0]))),
    ];
  });
  expect(result).toEqual([
    "Bijou32sDecodeError",
    "Bijou64sDecodeError",
    "Bijou128sDecodeError",
  ]);
});

test("decodeAllI128 returns an Array of bigints across the full range", async ({ page }) => {
  const result = await page.evaluate(() => {
    const b = (window as any).bijoux;
    const buf = new Uint8Array([
      ...b.encodeI128(-(2n ** 127n)),
      ...b.encodeI128(-1n),
      ...b.encodeI128(2n ** 127n - 1n),
    ]);
    const out = b.decodeAllI128(buf);
    return {
      isArray: Array.isArray(out),
      ok:
        out[0] === -(2n ** 127n) &&
        out[1] === -1n &&
        out[2] === 2n ** 127n - 1n,
    };
  });
  expect(result).toEqual({ isArray: true, ok: true });
});

test("i128 negative range bound throws RangeError", async ({ page }) => {
  const result = await page.evaluate(() => {
    const b = (window as any).bijoux;
    try {
      b.encodeI128(-(2n ** 127n) - 1n);
      return "no-throw";
    } catch (e: any) {
      return e.constructor.name;
    }
  });
  expect(result).toBe("RangeError");
});
