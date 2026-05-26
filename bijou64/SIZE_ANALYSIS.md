# bijou64 Encoded Size Analysis

> How many bytes each format uses to encode a given `u64` value.
>
> This is a property of the _format_, not the implementation -- it does
> not depend on architecture, compiler, or optimisation level. For
> wall-clock measurements of `encoded_len(v)`, see the per-arch shootout
> docs ([x86](SHOOTOUT_ANALYSIS_X86.md), [ARM](SHOOTOUT_ANALYSIS_ARM.md)).

## Format Recap

All tag-byte formats (bijou64, varu64, vu64/vu128) add one byte of
overhead for multi-byte values. leb128 uses one continuation bit per
byte instead.

bijou64 and varu64 share the same tag threshold (248), so their 1-byte
range is wider than vu64/vu128 (0--247 vs 0--127). bijou64's per-tier
offsets shift the multi-byte boundaries slightly, but the encoded sizes
end up identical to varu64 at most values -- with two important
exceptions (256--503 and 65,536--66,039), where bijou64's offset
correction lets it use _fewer_ bytes than varu64.

## Bytes Per Value

Each cell shows `bytes (% of raw u64)`. The leader (or leaders, in a
tie) for each row is bolded.

| Value      | Raw `u64` | bijou64           | varu64           | vu64 / vu128      | leb128            |
|------------|----------:|-------------------|------------------|-------------------|-------------------|
| 0          | 8         | **1 (12.5%)**     | **1 (12.5%)**    | **1 (12.5%)**     | **1 (12.5%)**     |
| 127        | 8         | **1 (12.5%)**     | **1 (12.5%)**    | **1 (12.5%)**     | **1 (12.5%)**     |
| 128        | 8         | **1 (12.5%)**     | **1 (12.5%)**    | 2 (25%)           | 2 (25%)           |
| 247        | 8         | **1 (12.5%)**     | **1 (12.5%)**    | 2 (25%)           | 2 (25%)           |
| 248        | 8         | **2 (25%)**       | **2 (25%)**      | **2 (25%)**       | **2 (25%)**       |
| 255        | 8         | **2 (25%)**       | **2 (25%)**      | **2 (25%)**       | **2 (25%)**       |
| 256        | 8         | **2 (25%)**       | 3 (37.5%)        | **2 (25%)**       | **2 (25%)**       |
| 503        | 8         | **2 (25%)**       | 3 (37.5%)        | **2 (25%)**       | **2 (25%)**       |
| 504        | 8         | 3 (37.5%)         | 3 (37.5%)        | **2 (25%)**       | **2 (25%)**       |
| 1,000      | 8         | 3 (37.5%)         | 3 (37.5%)        | **2 (25%)**       | **2 (25%)**       |
| 16,383     | 8         | 3 (37.5%)         | 3 (37.5%)        | **2 (25%)**       | **2 (25%)**       |
| 16,384     | 8         | **3 (37.5%)**     | **3 (37.5%)**    | **3 (37.5%)**     | **3 (37.5%)**     |
| 65,535     | 8         | **3 (37.5%)**     | **3 (37.5%)**    | **3 (37.5%)**     | **3 (37.5%)**     |
| 65,536     | 8         | **3 (37.5%)**     | 4 (50%)          | **3 (37.5%)**     | **3 (37.5%)**     |
| 66,039     | 8         | **3 (37.5%)**     | 4 (50%)          | **3 (37.5%)**     | **3 (37.5%)**     |
| 100,000    | 8         | 4 (50%)           | 4 (50%)          | **3 (37.5%)**     | **3 (37.5%)**     |
| 2^24 - 1   | 8         | **4 (50%)**       | **4 (50%)**      | **4 (50%)**       | **4 (50%)**       |
| 2^32 - 1   | 8         | **5 (62.5%)**     | **5 (62.5%)**    | **5 (62.5%)**     | **5 (62.5%)**     |
| 2^40 - 1   | 8         | **6 (75%)**       | **6 (75%)**      | **6 (75%)**       | **6 (75%)**       |
| 2^48 - 1   | 8         | **7 (87.5%)**     | **7 (87.5%)**    | **7 (87.5%)**     | **7 (87.5%)**     |
| 2^56 - 1   | 8         | **8 (100%)**      | **8 (100%)**     | **8 (100%)**      | **8 (100%)**      |
| 2^64 - 1   | 8         | **9 (112.5%)**    | **9 (112.5%)**   | **9 (112.5%)**    | 10 (125%)         |

## Tally

Counting shared wins (ties for first place each count as a win for
every tied format) across the 22 sample values:

| Format       | Wins | Notes                                                                 |
|--------------|-----:|-----------------------------------------------------------------------|
| vu64 / vu128 | 20   | Loses only to bijou64/varu64 on 128--247 (their 1-byte tier ends at 127) |
| leb128       | 19   | Same as vu64/vu128 except loses the 2^64 - 1 cell (10 bytes vs 9)     |
| bijou64      | 18   | Loses to vu64/vu128/leb128 on tier-2-heavy values (504--16,383, 100,000) |
| varu64       | 14   | Same shape as bijou64 but lacks the offset correction (loses 256--503 and 65,536--66,039) |

vu64/vu128 take the most cells overall thanks to their tight 2-byte
tier (128--16,383); leb128 trails by one cell at the high end. bijou64
edges out varu64 by 4 cells via the per-tier offset correction
(256--503 and 65,536--66,039), and matches or beats vu64/vu128/leb128
wherever the 1-byte tier extends past 127.

## Where bijou64 Loses

There is one regime where bijou64 is strictly worse than vu64/vu128/leb128:

| Range          | bijou64 | vu64/vu128/leb128 | Why                                              |
|----------------|--------:|------------------:|--------------------------------------------------|
| 504--16,383    | 3 bytes | 2 bytes           | bijou64's tier-2 range ends at 65,791; vu64 can pack 14 bits in 2 bytes here |
| 100,000        | 4 bytes | 3 bytes           | Same root cause, one tier higher                 |

These are the cost of structural canonicality: bijou64's tier ranges
are disjoint by construction, so it can't reuse the bit budget the way
formats with overlap can. For workloads dominated by these mid-range
values where size matters more than canonicality, leb128 or vu64 is the
right choice.

## Where bijou64 Beats Or Matches The Leader

| Range          | bijou64 | varu64  | vu64/vu128 | leb128   | Note                                                |
|----------------|--------:|--------:|-----------:|---------:|-----------------------------------------------------|
| 128--247       | 1 byte  | 1 byte  | 2 bytes    | 2 bytes  | bijou64 + varu64 share the lead (wider 1-byte tier) |
| 256--503       | 2 bytes | 3 bytes | 2 bytes    | 2 bytes  | bijou64 ties vu64/vu128/leb128; varu64 trails       |
| 65,536--66,039 | 3 bytes | 4 bytes | 3 bytes    | 3 bytes  | Same shape, one tier higher                         |
| 2^64 - 1       | 9 bytes | 9 bytes | 9 bytes    | 10 bytes | leb128's continuation bits cost an extra byte here  |

## License

This document is licensed CC BY-SA 4.0, matching `SPEC.md`.
