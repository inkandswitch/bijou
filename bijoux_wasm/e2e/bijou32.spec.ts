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
  await page.waitForFunction(() => (window as any).bijouxReady === true, {
    timeout: 30_000,
  });
});

test("MAX_BYTES_U32 is 5", async ({ page }) => {
  const max = await page.evaluate(() => (window as any).bijoux.MAX_BYTES_U32());
  expect(max).toBe(5);
});

test("encodes tier-0 values as a single byte equal to the value", async ({ page }) => {
  const result = await page.evaluate(() => {
    const { encodeU32 } = (window as any).bijoux;
    return {
      zero: [...encodeU32(0)],
      mid: [...encodeU32(42)],
      max: [...encodeU32(251)],
    };
  });
  expect(result.zero).toEqual([0x00]);
  expect(result.mid).toEqual([0x2a]);
  expect(result.max).toEqual([0xfb]);
});

test("encodes tier-1 values with offset", async ({ page }) => {
  const result = await page.evaluate(() => {
    const { encodeU32 } = (window as any).bijoux;
    return {
      lower: [...encodeU32(252)],
      mid: [...encodeU32(300)],
      upper: [...encodeU32(507)],
    };
  });
  expect(result.lower).toEqual([0xfc, 0x00]);
  expect(result.mid).toEqual([0xfc, 0x30]);
  expect(result.upper).toEqual([0xfc, 0xff]);
});

test("encodes u32::MAX as 5 bytes starting with 0xFF", async ({ page }) => {
  const result = await page.evaluate(() => {
    const { encodeU32 } = (window as any).bijoux;
    const bytes = encodeU32(2 ** 32 - 1);
    return { length: bytes.length, firstByte: bytes[0] };
  });
  expect(result.length).toBe(5);
  expect(result.firstByte).toBe(0xff);
});

test("encodedLenU32 agrees with encodeU32().length across tier boundaries", async ({ page }) => {
  const result = await page.evaluate(() => {
    const { encodeU32, encodedLenU32 } = (window as any).bijoux;
    const cases: number[] = [
      0, 251, 252, 507, 508, 65_535, 66_043, 66_044,
      16_843_259, 16_843_260, 2 ** 32 - 2, 2 ** 32 - 1,
    ];
    return cases.map((v) => ({
      value: v,
      computed: encodedLenU32(v),
      actual: encodeU32(v).length,
    }));
  });
  for (const row of result) {
    expect(row.computed, `encodedLenU32(${row.value})`).toBe(row.actual);
  }
});

test("round-trips every tier boundary", async ({ page }) => {
  const result = await page.evaluate(() => {
    const { encodeU32, decodeU32 } = (window as any).bijoux;
    const cases: number[] = [
      0, 1, 251, 252, 507, 508, 65_535, 66_043, 66_044,
      16_843_259, 16_843_260, 2 ** 32 - 2, 2 ** 32 - 1,
    ];
    return cases.map((v) => {
      const bytes = encodeU32(v);
      const r = decodeU32(bytes);
      return {
        input: v,
        decoded: r.value,
        bytesRead: r.bytesRead,
        encodedLenU32: bytes.length,
      };
    });
  });
  for (const row of result) {
    expect(row.decoded, `round-trip ${row.input}`).toBe(row.input);
    expect(row.bytesRead).toBe(row.encodedLenU32);
  }
});

test("decodeU32 reports bytesRead < input length when buffer has trailing data", async ({ page }) => {
  const result = await page.evaluate(() => {
    const { encodeU32, decodeU32 } = (window as any).bijoux;
    const head = encodeU32(300); // 2 bytes
    const buf = new Uint8Array(head.length + 3);
    buf.set(head, 0);
    buf.set([0xaa, 0xbb, 0xcc], head.length);
    const r = decodeU32(buf);
    return { value: r.value, bytesRead: r.bytesRead, inputLength: buf.length };
  });
  expect(result.value).toBe(300);
  expect(result.bytesRead).toBe(2);
  expect(result.inputLength).toBe(5);
});

test("decodeU32 throws Bijou32DecodeError on empty input", async ({ page }) => {
  const result = await page.evaluate(() => {
    const { decodeU32 } = (window as any).bijoux;
    try {
      decodeU32(new Uint8Array([]));
      return { threw: false } as const;
    } catch (e: any) {
      return { threw: true, name: e.name, message: e.message } as const;
    }
  });
  expect(result.threw).toBe(true);
  expect(result.name).toBe("Bijou32DecodeError");
  expect(result.message).toContain("buffer too short");
});

test("decodeU32 throws Bijou32DecodeError on truncated tier-4 input", async ({ page }) => {
  const result = await page.evaluate(() => {
    const { decodeU32 } = (window as any).bijoux;
    try {
      // 0xFF needs 4 payload bytes; supply 3.
      decodeU32(new Uint8Array([0xff, 0, 0, 0]));
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
    const { decodeU32 } = (window as any).bijoux;
    try {
      decodeU32(new Uint8Array([0xfc]));
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

test("decodeU32 throws TypeError on non-Uint8Array input (no silent truncation)", async ({ page }) => {
  // Guards the shipped dist against the silent-truncation footgun in a
  // real browser: a plain JS Array would be coerced via
  // `new Uint8Array(arr)`, truncating out-of-range elements
  // (1000 & 0xFF === 232). decodeU32/decodeAllU32 must reject anything that
  // isn't a real Uint8Array.
  const result = await page.evaluate(() => {
    const { decodeU32, decodeAllU32 } = (window as any).bijoux;
    const bad: unknown[] = [[1000], [0x00], null, 42, "nope"];
    const out = { fn: "", allTypeError: true };
    for (const input of bad) {
      for (const [name, f] of [
        ["decodeU32", decodeU32],
        ["decodeAllU32", decodeAllU32],
      ] as const) {
        try {
          f(input);
          out.fn = name;
          out.allTypeError = false;
        } catch (e: any) {
          if (!(e instanceof TypeError)) {
            out.fn = name;
            out.allTypeError = false;
          }
        }
      }
    }
    return out;
  });
  expect(result.allTypeError, `failing fn: ${result.fn}`).toBe(true);
});

test("encodeU32 throws RangeError for numbers >= 2**32", async ({ page }) => {
  const result = await page.evaluate(() => {
    const { encodeU32 } = (window as any).bijoux;
    try {
      encodeU32(2 ** 32);
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

test("encodeU32 throws RangeError for negative numbers", async ({ page }) => {
  const result = await page.evaluate(() => {
    const { encodeU32, encodedLenU32 } = (window as any).bijoux;
    const cases = [
      () => encodeU32(-1),
      () => encodeU32(-(2 ** 31)),
      () => encodedLenU32(-1),
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

test("encodeU32 throws TypeError for fractional, NaN, Infinity, or non-number inputs", async ({ page }) => {
  const result = await page.evaluate(() => {
    const { encodeU32, encodedLenU32 } = (window as any).bijoux;
    const cases = [
      () => encodeU32(1.5),
      () => encodeU32(0.1),
      () => encodeU32(NaN),
      () => encodeU32(Infinity),
      () => encodeU32(-Infinity),
      () => encodeU32(42n),       // bigint, not number
      () => encodeU32("300"),
      () => encodeU32(null),
      () => encodeU32(undefined),
      () => encodedLenU32({} as any),
      () => encodedLenU32(3.14),
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

test("decodeAllU32 returns a Uint32Array of every value in the buffer", async ({ page }) => {
  const result = await page.evaluate(() => {
    const { encodeU32, decodeAllU32 } = (window as any).bijoux;
    const merged = new Uint8Array([
      ...encodeU32(42),
      ...encodeU32(300),
      ...encodeU32(65535),
      ...encodeU32(1 << 24),
    ]);
    const values = decodeAllU32(merged);
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

test("decodeAllU32 on an empty buffer returns an empty Uint32Array", async ({ page }) => {
  const result = await page.evaluate(() => {
    const { decodeAllU32 } = (window as any).bijoux;
    const empty = decodeAllU32(new Uint8Array(0));
    return { typeName: empty.constructor.name, length: empty.length };
  });
  expect(result.typeName).toBe("Uint32Array");
  expect(result.length).toBe(0);
});

test("decodeAllU32 throws Bijou32DecodeError on a malformed element", async ({ page }) => {
  const result = await page.evaluate(() => {
    const { decodeAllU32 } = (window as any).bijoux;
    try {
      decodeAllU32(new Uint8Array([0x42, 0xfc]));
      return { threw: false } as const;
    } catch (e: any) {
      return { threw: true, name: e.name, isError: e instanceof Error } as const;
    }
  });
  expect(result.threw).toBe(true);
  expect(result.name).toBe("Bijou32DecodeError");
  expect(result.isError).toBe(true);
});

test("encodeU32 accepts the boundary values 0 and u32::MAX exactly", async ({ page }) => {
  const result = await page.evaluate(() => {
    const { encodeU32 } = (window as any).bijoux;
    return {
      zero: [...encodeU32(0)],
      max: [...encodeU32(2 ** 32 - 1)],
    };
  });
  expect(result.zero).toEqual([0x00]);
  expect(result.max).toEqual([0xff, 0xfe, 0xfe, 0xfe, 0x03]);
});
