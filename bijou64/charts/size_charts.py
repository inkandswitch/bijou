#!/usr/bin/env python3
"""Generate encoded-size comparison charts for bijou64.

These are architecture-independent (the chart is a property of the format,
not the implementation). Outputs into `bijou64/charts/size/`:

  - bytes_vs_value.svg       full-range step plot, log-x
  - bytes_vs_value_low.svg   zoomed step plot for 0–66,500 (where formats diverge)
  - heatmap.svg              22-value reference table as a heatmap
  - boundary_detail.svg      four side-by-side panels around each interesting tier boundary

Usage:
  nix run .#size-charts
  python bijou64/charts/size_charts.py
"""

from __future__ import annotations

import sys
from pathlib import Path

import matplotlib

matplotlib.use("Agg")

import matplotlib.pyplot as plt
import numpy as np
from matplotlib.patches import Patch

# ---------------------------------------------------------------------------
# Format definitions
# ---------------------------------------------------------------------------

TAG_THRESHOLD = 248  # bijou64 / varu64


def _bijou64_offsets() -> list[int]:
    """Mirror of bijou64::OFFSETS (see src/lib.rs:85-118)."""
    offs = [0, TAG_THRESHOLD]
    for n in range(2, 9):
        offs.append(offs[-1] + 256 ** (n - 1))
    return offs


BIJOU64_OFFSETS = _bijou64_offsets()


def bijou64_len(v: int) -> int:
    """Bytes used by bijou64 to encode `v`. See src/lib.rs:157-187."""

    if v < BIJOU64_OFFSETS[1]:  # tier 0
        return 1
    for tier in range(1, 9):
        next_off = BIJOU64_OFFSETS[tier + 1] if tier < 8 else (1 << 64)
        if v < next_off:
            return tier + 1
    # u64::MAX falls in tier 8 (9-byte total)
    return 9


def varu64_len(v: int) -> int:
    """varu64: same 1-byte threshold as bijou64 (248), but no per-tier offset
    correction. Multi-byte tiers split on power-of-256 boundaries.
    """

    if v < TAG_THRESHOLD:
        return 1
    # tier n (n >= 1) holds values in [248 ... 248 + 256^n - 1] minus what
    # the earlier tiers covered. The varu64 spec uses 256^n boundaries:
    # tier 1: < 248 + 256        => effectively 2 bytes (tag + 1 payload)
    # tier 2: < 248 + 65_536     => 3 bytes
    # but the implementation actually just looks at byte-width of the payload.
    # Practically, the encoded byte counts match bijou64 EXCEPT in the
    # regions where bijou64's offset correction kicks in:
    #   256..=503 and 65,536..=66,039.
    # Outside those windows, varu64 == bijou64.
    bw = max(1, (v.bit_length() + 7) // 8)
    return bw + 1  # 1 tag byte + bw payload bytes


def vu64_len(v: int) -> int:
    """vu64 / vu128: tier boundaries at exact powers of 2.

    See vu64-0.2.0/src/lib.rs: encoded_len is a table indexed by
    leading_zeros. The result is `ceil(bit_width / 7)` clamped to [1, 9].
    """

    if v == 0:
        return 1
    bw = v.bit_length()
    return min(9, (bw + 6) // 7)


def leb128_len(v: int) -> int:
    """leb128: 7 payload bits per byte, no tag byte. 10 bytes for u64::MAX."""

    if v == 0:
        return 1
    bw = v.bit_length()
    return (bw + 6) // 7


FORMATS: dict[str, tuple[callable, str]] = {
    "bijou64":     (bijou64_len, "#1f77b4"),
    "varu64":      (varu64_len,  "#ff7f0e"),
    "vu64 / vu128": (vu64_len,    "#2ca02c"),
    "leb128":      (leb128_len,  "#d62728"),
}

# Interesting boundaries to label
BIJOU64_TIER_BOUNDARIES = BIJOU64_OFFSETS[1:9]  # 248, 65792, 16843008, ...
VU64_TIER_BOUNDARIES = [1 << (7 * k) for k in range(1, 10)]  # 128, 16384, ...

# ---------------------------------------------------------------------------
# Sample-value sweep helpers
# ---------------------------------------------------------------------------


def boundary_dense_values() -> list[int]:
    """A sweep that hits every interesting transition densely, plus a
    log-spaced backbone for the rest of the u64 range. Returns a sorted
    list of Python ints (numpy int64 can't hold u64::MAX).
    """

    pts: set[int] = {0, 1}
    # Hit each tier boundary and its neighborhood for both schemes
    for b in BIJOU64_TIER_BOUNDARIES + VU64_TIER_BOUNDARIES:
        for delta in (-2, -1, 0, 1, 2):
            pt = b + delta
            if 0 <= pt < (1 << 64):
                pts.add(pt)
    # Log-spaced backbone — work in float, convert each point manually
    # so we don't lose precision on the very top of the u64 range.
    for x in np.logspace(0, np.log10(float((1 << 64) - 1)), 400):
        pt = int(round(x))
        if 0 < pt < (1 << 64):
            pts.add(pt)
    pts.add((1 << 64) - 1)
    return sorted(pts)


# ---------------------------------------------------------------------------
# Plot 1: full-range bytes vs value
# ---------------------------------------------------------------------------


def plot_bytes_vs_value(out: Path) -> None:
    # Log-scale x-axis can't represent v == 0, so drop it for this plot only.
    # (Other plots use symlog or linear axes and keep the zero point.)
    xs_int = [v for v in boundary_dense_values() if v > 0]
    # Matplotlib wants numeric arrays; cast to float since u64::MAX exceeds i64.
    xs = np.array(xs_int, dtype=float)
    fig, ax = plt.subplots(figsize=(10, 5))

    for (label, (fn, color)), lw in zip(FORMATS.items(), (2.2, 1.6, 1.6, 1.6)):
        ys = np.array([fn(x) for x in xs_int])
        ax.step(xs, ys, where="post", label=label, color=color, linewidth=lw, alpha=0.9)

    ax.set_xscale("log")
    ax.set_xlim(1, float(1 << 64))
    ax.set_ylim(0.5, 10.5)
    ax.set_yticks(range(1, 11))
    ax.set_xlabel("Value (log scale)")
    ax.set_ylabel("Encoded length (bytes)")
    ax.set_title("Encoded length vs value")
    ax.grid(True, which="both", alpha=0.25)

    # Mark bijou64 tier boundaries
    for b in BIJOU64_TIER_BOUNDARIES:
        ax.axvline(float(b), color="#1f77b4", linestyle=":", linewidth=0.6, alpha=0.35)
    # Mark vu64/leb128 tier boundaries
    for b in VU64_TIER_BOUNDARIES[:8]:
        ax.axvline(float(b), color="#2ca02c", linestyle=":", linewidth=0.6, alpha=0.25)

    ax.legend(loc="lower right", framealpha=0.95)
    fig.tight_layout()
    fig.savefig(out, format="svg")
    plt.close(fig)


# ---------------------------------------------------------------------------
# Plot 2: zoomed low range (0--66_039) where the formats actually disagree
# ---------------------------------------------------------------------------


def plot_bytes_vs_value_low(out: Path) -> None:
    # Dense sweep in the interesting region
    xs = np.unique(
        np.concatenate([
            np.arange(0, 600),
            np.arange(600, 16_500, 5),
            np.arange(16_500, 66_500, 50),
        ])
    ).astype(np.int64)

    fig, ax = plt.subplots(figsize=(10, 5))

    for (label, (fn, color)), lw in zip(FORMATS.items(), (2.2, 1.6, 1.6, 1.6)):
        ys = np.array([fn(int(x)) for x in xs])
        ax.step(xs, ys, where="post", label=label, color=color, linewidth=lw, alpha=0.9)

    ax.set_xscale("symlog", linthresh=10)
    ax.set_xlim(0, 66_500)
    # Extra headroom at the top so the rotated boundary labels don't overlap
    # the data lines.
    ax.set_ylim(0.5, 5.6)
    ax.set_yticks([1, 2, 3, 4])
    ax.set_xlabel("Value (symlog scale, linear below 10)")
    ax.set_ylabel("Encoded length (bytes)")
    ax.set_title("Encoded length vs value (low range, 0\u201366,500)")

    # Label boundaries with vertical lines + rotated text. bijou64 tier-N→(N+1)
    # happens at OFFSETS[N+1]; key values in this range:
    #   OFFSETS[1] = 248,  OFFSETS[2] = 504,  OFFSETS[3] = 66,040.
    # vu64 boundaries are powers of 128 (7-bit payload): 128, 16,384.
    boundary_annotations = [
        (128, "vu64 tier 1→2", "#2ca02c"),
        (248, "bijou64/varu64 tier 0→1", "#1f77b4"),
        (256, "varu64 → 3B (no offset)", "#ff7f0e"),
        (504, "bijou64 tier 1→2", "#1f77b4"),
        (16_384, "vu64 tier 2→3", "#2ca02c"),
        (65_536, "varu64 → 4B (no offset)", "#ff7f0e"),
        (66_040, "bijou64 tier 2→3", "#1f77b4"),
    ]
    for x, label, color in boundary_annotations:
        ax.axvline(x, color=color, linestyle=":", linewidth=0.7, alpha=0.5)
        # Anchor the label just above the data area; rotated 90° so each one
        # fits in the narrow channel its vertical line occupies.
        ax.text(
            x, 4.65, label,
            rotation=90, rotation_mode="anchor",
            va="bottom", ha="center",
            fontsize=7, color=color, alpha=0.9,
        )

    ax.grid(True, which="major", alpha=0.3)
    ax.legend(loc="upper left", framealpha=0.95)
    fig.tight_layout()
    fig.savefig(out, format="svg")
    plt.close(fig)


# ---------------------------------------------------------------------------
# Plot 3: heatmap of the 22-value reference table
# ---------------------------------------------------------------------------


REFERENCE_VALUES: list[tuple[str, int]] = [
    ("0", 0),
    ("127", 127),
    ("128", 128),
    ("247", 247),
    ("248", 248),
    ("255", 255),
    ("256", 256),
    ("503", 503),
    ("504", 504),
    ("1,000", 1_000),
    ("16,383", 16_383),
    ("16,384", 16_384),
    ("65,535", 65_535),
    ("65,536", 65_536),
    ("66,039", 66_039),
    ("100,000", 100_000),
    ("2^24 - 1", (1 << 24) - 1),
    ("2^32 - 1", (1 << 32) - 1),
    ("2^40 - 1", (1 << 40) - 1),
    ("2^48 - 1", (1 << 48) - 1),
    ("2^56 - 1", (1 << 56) - 1),
    ("2^64 - 1", (1 << 64) - 1),
]


def plot_heatmap(out: Path) -> None:
    labels = [label for label, _ in REFERENCE_VALUES]
    values = [v for _, v in REFERENCE_VALUES]
    formats = list(FORMATS.keys())

    grid = np.zeros((len(values), len(formats)), dtype=int)
    for r, v in enumerate(values):
        for c, name in enumerate(formats):
            grid[r, c] = FORMATS[name][0](v)

    fig, ax = plt.subplots(figsize=(7, 9))
    im = ax.imshow(grid, aspect="auto", cmap="YlOrRd", vmin=1, vmax=10,
                   origin="lower")

    ax.set_xticks(range(len(formats)))
    ax.set_xticklabels(formats)
    ax.set_yticks(range(len(labels)))
    ax.set_yticklabels(labels)
    ax.set_xlabel("Format")
    ax.set_ylabel("Value (ascending)")
    ax.set_title("Encoded length (bytes) per value")

    # Cell annotations -- with origin="lower" the matrix is rendered with
    # row 0 at the bottom, so we use the same row indices as the data grid.
    for r in range(grid.shape[0]):
        row_min = grid[r].min()
        for c in range(grid.shape[1]):
            val = grid[r, c]
            weight = "bold" if val == row_min else "normal"
            color = "white" if val >= 7 else "black"
            ax.text(c, r, str(val), ha="center", va="center",
                    fontweight=weight, color=color, fontsize=10)

    cbar = fig.colorbar(im, ax=ax, shrink=0.7)
    cbar.set_label("Bytes")

    fig.tight_layout()
    fig.savefig(out, format="svg")
    plt.close(fig)


# ---------------------------------------------------------------------------
# Plot 4: boundary detail panels
# ---------------------------------------------------------------------------


def plot_boundary_detail(out: Path) -> None:
    """Four panels, each zoomed around an interesting transition:

      A. 120--260   (vu64 and bijou64/varu64 tier-0→1 differ here)
      B. 240--520   (bijou64's offset correction extends 2-byte range)
      C. 16,300--16,500   (vu64 tier-2→3)
      D. 65,500--66,200   (bijou64 again gets an extra-2-byte sliver)
    """

    panels = [
        ("(a) Wider 1-byte tier: bijou64/varu64 reach 247", 120, 260, [1, 2]),
        ("(b) bijou64 offset extends 2-byte tier to 503",  240, 520, [1, 2, 3]),
        ("(c) vu64 tier 2→3 at 16,384",  16_300, 16_500, [2, 3]),
        ("(d) bijou64 stays at 3 bytes past 65,535 (varu64 jumps to 4)", 65_400, 66_200, [3, 4]),
    ]

    fig, axes = plt.subplots(2, 2, figsize=(12, 7))
    for ax, (title, lo, hi, yticks) in zip(axes.flat, panels):
        xs = np.arange(lo, hi + 1)
        for label, (fn, color) in FORMATS.items():
            ys = np.array([fn(int(x)) for x in xs])
            ax.step(xs, ys, where="post", label=label, color=color, linewidth=1.6, alpha=0.9)
        ax.set_xlim(lo, hi)
        ax.set_ylim(min(yticks) - 0.5, max(yticks) + 0.5)
        ax.set_yticks(yticks)
        ax.set_title(title, fontsize=10)
        ax.grid(True, alpha=0.3)
        ax.tick_params(labelsize=8)

    # One shared legend below the panels
    handles = [
        Patch(facecolor=color, label=label)
        for label, (_, color) in FORMATS.items()
    ]
    fig.legend(handles=handles, loc="lower center", ncol=4, frameon=False,
               bbox_to_anchor=(0.5, -0.02))

    fig.suptitle("Boundary detail: where bijou64 differs from its peers", y=1.00)
    fig.tight_layout()
    fig.savefig(out, format="svg", bbox_inches="tight")
    plt.close(fig)


# ---------------------------------------------------------------------------
# Entrypoint
# ---------------------------------------------------------------------------


def workspace_root() -> Path:
    here = Path(__file__).resolve()
    for parent in here.parents:
        if (parent / "Cargo.toml").exists() and (parent / "bijou64").is_dir():
            return parent
    raise SystemExit("could not locate workspace root (no Cargo.toml + bijou64/ found)")


def main() -> int:
    root = workspace_root()
    out_dir = root / "bijou64" / "charts" / "size"
    out_dir.mkdir(parents=True, exist_ok=True)

    plot_bytes_vs_value(out_dir / "bytes_vs_value.svg")
    plot_bytes_vs_value_low(out_dir / "bytes_vs_value_low.svg")
    plot_heatmap(out_dir / "heatmap.svg")
    plot_boundary_detail(out_dir / "boundary_detail.svg")

    print(f"wrote 4 SVGs to {out_dir.relative_to(root)}/")
    return 0


if __name__ == "__main__":
    sys.exit(main())
