import { test, expect, type Page } from "@playwright/test";

/**
 * Cross-browser smoke tests for the published `bijou32` npm package.
 *
 * These tests deliberately exercise the package _through `window.bijou32`_
 * (i.e. the same way a downstream consumer would use it) rather than
 * importing the Rust types directly. That catches regressions in any of:
 *
 * - the wasm-bindgen ABI (Number ↔ u32 marshalling, Uint8Array ↔ &[u8])
 * - the wasm-bodge "web" entrypoint (base64 init path)
 * - the package.json subpath exports
 * - cross-browser Number + Uint8Array support
 *
 * Unlike `bijou64`/`bijou128`, `bijou32` uses plain JS `number` at the
 * boundary because `u32::MAX` fits in `Number.MAX_SAFE_INTEGER`.
 */

const SERVER_URL = "/e2e/server/index.html";

test.beforeEach(async ({ page }: { page: Page }) => {
  await page.goto(SERVER_URL);
  await page.waitForFunction(() => (window as any).bijou32Ready === true, {
    timeout: 30_000,
  });
});

test("MAX_BYTES is 5", async ({ page }) => {
  const max = await page.evaluate(() => (window as any).bijou32.MAX_BYTES());
  expect(max).toBe(5);
});

test("encodes tier-0 values as a single byte equal to the value", async ({ page }) => {
  const result = await page.evaluate(() => {
    const { encode } = (window as any).bijou32;
    return {
      zero: [...encode(0)],
      mid: [...encode(42)],
      max: [...encode(251)],
    };
  });
  expect(result.zero).toEqual([0x00]);
  expect(result.mid).toEqual([0x2a]);
  expect(result.max).toEqual([0xfb]);
});

test("encodes tier-1 values with offset", async ({ page }) => {
  const result = await page.evaluate(() => {
    const { encode } = (window as any).bijou32;
    return {
      lower: [...encode(252)],
      mid: [...encode(300)],
      upper: [...encode(507)],
    };
  });
  expect(result.lower).toEqual([0xfc, 0x00]);
  expect(result.mid).toEqual([0xfc, 0x30]);
  expect(result.upper).toEqual([0xfc, 0xff]);
});

test("encodes u32::MAX as 5 bytes starting with 0xFF", async ({ page }) => {
  const result = await page.evaluate(() => {
    const { encode } = (window as any).bijou32;
    const bytes = encode(2 ** 32 - 1);
    return { length: bytes.length, firstByte: bytes[0] };
  });
  expect(result.length).toBe(5);
  expect(result.firstByte).toBe(0xff);
});

test("encodedLen agrees with encode().length across tier boundaries", async ({ page }) => {
  const result = await page.evaluate(() => {
    const { encode, encodedLen } = (window as any).bijou32;
    const cases: number[] = [
      0, 251, 252, 507, 508, 65_535, 66_043, 66_044,
      16_843_259, 16_843_260, 2 ** 32 - 2, 2 ** 32 - 1,
    ];
    return cases.map((v) => ({
      value: v,
      computed: encodedLen(v),
      actual: encode(v).length,
    }));
  });
  for (const row of result) {
    expect(row.computed, `encodedLen(${row.value})`).toBe(row.actual);
  }
});

test("round-trips every tier boundary", async ({ page }) => {
  const result = await page.evaluate(() => {
    const { encode, decode } = (window as any).bijou32;
    const cases: number[] = [
      0, 1, 251, 252, 507, 508, 65_535, 66_043, 66_044,
      16_843_259, 16_843_260, 2 ** 32 - 2, 2 ** 32 - 1,
    ];
    return cases.map((v) => {
      const bytes = encode(v);
      const r = decode(bytes);
      return {
        input: v,
        decoded: r.value,
        bytesRead: r.bytesRead,
        encodedLen: bytes.length,
      };
    });
  });
  for (const row of result) {
    expect(row.decoded, `round-trip ${row.input}`).toBe(row.input);
    expect(row.bytesRead).toBe(row.encodedLen);
  }
});

test("decode reports bytesRead < input length when buffer has trailing data", async ({ page }) => {
  const result = await page.evaluate(() => {
    const { encode, decode } = (window as any).bijou32;
    const head = encode(300); // 2 bytes
    const buf = new Uint8Array(head.length + 3);
    buf.set(head, 0);
    buf.set([0xaa, 0xbb, 0xcc], head.length);
    const r = decode(buf);
    return { value: r.value, bytesRead: r.bytesRead, inputLength: buf.length };
  });
  expect(result.value).toBe(300);
  expect(result.bytesRead).toBe(2);
  expect(result.inputLength).toBe(5);
});

test("decode throws Bijou32DecodeError on empty input", async ({ page }) => {
  const result = await page.evaluate(() => {
    const { decode } = (window as any).bijou32;
    try {
      decode(new Uint8Array([]));
      return { threw: false } as const;
    } catch (e: any) {
      return { threw: true, name: e.name, message: e.message } as const;
    }
  });
  expect(result.threw).toBe(true);
  expect(result.name).toBe("Bijou32DecodeError");
  expect(result.message).toContain("buffer too short");
});

test("decode throws Bijou32DecodeError on truncated tier-4 input", async ({ page }) => {
  const result = await page.evaluate(() => {
    const { decode } = (window as any).bijou32;
    try {
      // 0xFF needs 4 payload bytes; supply 3.
      decode(new Uint8Array([0xff, 0, 0, 0]));
      return { threw: false } as const;
    } catch (e: any) {
      return { threw: true, name: e.name } as const;
    }
  });
  expect(result.threw).toBe(true);
  expect(result.name).toBe("Bijou32DecodeError");
});

test("Bijou32DecodeError thrown is an instance of the platform Error", async ({ page }) => {
  const result = await page.evaluate(() => {
    const { decode } = (window as any).bijou32;
    try {
      decode(new Uint8Array([0xfc]));
      return { threw: false } as const;
    } catch (e: any) {
      return {
        threw: true,
        isError: e instanceof Error,
        hasStack: typeof e.stack === "string" && e.stack.length > 0,
      } as const;
    }
  });
  expect(result.threw).toBe(true);
  expect(result.isError).toBe(true);
  expect(result.hasStack).toBe(true);
});

test("encode throws RangeError for numbers >= 2**32", async ({ page }) => {
  const result = await page.evaluate(() => {
    const { encode } = (window as any).bijou32;
    try {
      encode(2 ** 32);
      return { threw: false } as const;
    } catch (e: any) {
      return {
        threw: true,
        name: e.name,
        isError: e instanceof Error,
        messageHasRange: typeof e.message === "string" && e.message.includes("2**32"),
      } as const;
    }
  });
  expect(result.threw).toBe(true);
  expect(result.name).toBe("RangeError");
  expect(result.isError).toBe(true);
  expect(result.messageHasRange).toBe(true);
});

test("encode throws RangeError for negative numbers", async ({ page }) => {
  const result = await page.evaluate(() => {
    const { encode, encodedLen } = (window as any).bijou32;
    const cases = [
      () => encode(-1),
      () => encode(-(2 ** 31)),
      () => encodedLen(-1),
    ];
    return cases.map((fn) => {
      try {
        fn();
        return { threw: false } as const;
      } catch (e: any) {
        return { threw: true, name: e.name } as const;
      }
    });
  });
  for (const r of result) {
    expect(r.threw).toBe(true);
    expect(r.name).toBe("RangeError");
  }
});

test("encode throws TypeError for fractional, NaN, Infinity, or non-number inputs", async ({ page }) => {
  const result = await page.evaluate(() => {
    const { encode, encodedLen } = (window as any).bijou32;
    const cases = [
      () => encode(1.5),
      () => encode(0.1),
      () => encode(NaN),
      () => encode(Infinity),
      () => encode(-Infinity),
      () => encode(42n),       // bigint, not number
      () => encode("300"),
      () => encode(null),
      () => encode(undefined),
      () => encodedLen({} as any),
      () => encodedLen(3.14),
    ];
    return cases.map((fn) => {
      try {
        fn();
        return { threw: false } as const;
      } catch (e: any) {
        return {
          threw: true,
          name: e.name,
          isError: e instanceof Error,
        } as const;
      }
    });
  });
  for (const r of result) {
    expect(r.threw).toBe(true);
    expect(r.name).toBe("TypeError");
    expect(r.isError).toBe(true);
  }
});

test("decodeAll returns a Uint32Array of every value in the buffer", async ({ page }) => {
  const result = await page.evaluate(() => {
    const { encode, decodeAll } = (window as any).bijou32;
    const merged = new Uint8Array([
      ...encode(42),
      ...encode(300),
      ...encode(65535),
      ...encode(1 << 24),
    ]);
    const values = decodeAll(merged);
    return {
      typeName: values.constructor.name,
      length: values.length,
      isUint32Array: values instanceof Uint32Array,
      asArray: Array.from(values),
    };
  });
  expect(result.typeName).toBe("Uint32Array");
  expect(result.isUint32Array).toBe(true);
  expect(result.length).toBe(4);
  expect(result.asArray).toEqual([42, 300, 65535, 1 << 24]);
});

test("decodeAll on an empty buffer returns an empty Uint32Array", async ({ page }) => {
  const result = await page.evaluate(() => {
    const { decodeAll } = (window as any).bijou32;
    const empty = decodeAll(new Uint8Array(0));
    return { typeName: empty.constructor.name, length: empty.length };
  });
  expect(result.typeName).toBe("Uint32Array");
  expect(result.length).toBe(0);
});

test("decodeAll throws Bijou32DecodeError on a malformed element", async ({ page }) => {
  const result = await page.evaluate(() => {
    const { decodeAll } = (window as any).bijou32;
    try {
      decodeAll(new Uint8Array([0x42, 0xfc]));
      return { threw: false } as const;
    } catch (e: any) {
      return { threw: true, name: e.name, isError: e instanceof Error } as const;
    }
  });
  expect(result.threw).toBe(true);
  expect(result.name).toBe("Bijou32DecodeError");
  expect(result.isError).toBe(true);
});

test("encode accepts the boundary values 0 and u32::MAX exactly", async ({ page }) => {
  const result = await page.evaluate(() => {
    const { encode } = (window as any).bijou32;
    return {
      zero: [...encode(0)],
      max: [...encode(2 ** 32 - 1)],
    };
  });
  expect(result.zero).toEqual([0x00]);
  expect(result.max).toEqual([0xff, 0xfe, 0xfe, 0xfe, 0x03]);
});
