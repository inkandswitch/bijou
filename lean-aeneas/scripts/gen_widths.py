"""Derive the u32/u128 refinement proofs from the 64-bit ones.

The widths' Rust (and so their Aeneas translations) differ only in constants
and in the names Aeneas gives local variables. So:

- U32.lean / U128.lean are U64.lean with those substituted, *except*
  `decode_spec`, whose shape depends on the tier count (one `match` arm per
  tier). `decode_spec` is generated for all three widths -- U64.lean
  included, rewritten in place -- from the template in `decode_arm` /
  `decode_block` below. Edit the template, not any `decode_spec`.
- U32Corollaries / U128Corollaries from U64Corollaries.

CI (`ci-aeneas`) re-runs this script and fails if any output differs.

    python3 lean-aeneas/scripts/gen_widths.py lean-aeneas/BijouAeneas/Refinement
"""
import re, sys
SRC = sys.argv[1]
src = open(SRC + '/U64.lean').read()
MARK = '-- @@decode_spec@@\n'

def split_decode(text):
    """(before, after) the `decode_spec` block of a refinement file."""
    i = text.index('theorem decode_spec (buf')
    m = re.compile(r'^(theorem|lemma|def|@\[|/--|/-!|set_option|end|namespace|section|--) ', re.M).search(text, text.index('\n', i))
    j = m.start() if m else len(text)
    k = text.rfind('\n\n', 0, i)  # keep a preceding doc comment / options with the block
    return text[:k + 2], text[j:]

pre64, post64 = split_decode(src)
headA = pre64 + MARK + post64

W = {
  'u64':  dict(N=8,  T=248, bits=64,  elems='eight_elems',
               names='a1 b c d e f g h'.split(), add=lambda t: 'o2'),
  'u32':  dict(N=4,  T=252, bits=32,  elems='four_elems',
               names='a1 b c d'.split(), add=lambda t: 'o2'),
  'u128': dict(N=16, T=240, bits=128, elems='sixteen_elems',
               names='a1 b c d e f g h i j k l m n o2 p1'.split(),
               add=lambda t: 'o3' if t >= 15 else 'o2'),
}

def part_a(w, P):
    N, T, B = P['N'], P['T'], P['bits']
    s = headA
    U, u = w.upper(), w
    # width-1 numerals first (before generic renames touch them)
    s = s.replace('18446744073709551616', str(2**B)).replace('18446744073709551615', str(2**B - 1))
    s = s.replace('2 ^ 64', f'2 ^ {B}').replace('< 64', f'< {B}').replace('63 - Nat.log', f'{B-1} - Nat.log')
    # threshold / tag arithmetic
    s = s.replace('248', str(T)).replace('247', str(T-1)).replace('246', str(T-2))
    # tier count, only where 8 means "tiers"
    for a, b in [('n.val ≤ 8', f'n.val ≤ {N}'), ('256 ^ 7', f'256 ^ {N-1}'), ('256 ^ 8', f'256 ^ {N}'), ('(256 : ℕ) ^ 8 =', f'(256 : ℕ) ^ {N} ='),
                 ('∀ i < 8', f'∀ i < {N}'), ('∀ i ≤ 8', f'∀ i ≤ {N}'), ('tierOf v ≤ 8', f'tierOf v ≤ {N}'),
                 ('buf.length + 9', f'buf.length + {N+1}'), ('NUM_TIERS.val = 8', f'NUM_TIERS.val = {N}'),
                 ('8 * (8 - ', f'8 * ({N} - '), ('(8 - Bijou', f'({N} - Bijou'), ('= 8 by omega', f'= {N} by omega'),
                 ('pb.val.length = 8', f'pb.val.length = {N}'), ('e.len.val ≤ 9', f'e.len.val ≤ {N+1}'), ('tiers `≤ 8`', f'tiers `≤ {N}`'),
                 ('offset 1 .. offset 8', f'offset 1 .. offset {N}'), ('eight_elems', P['elems'])]:
        assert a in s, (w, a)
        s = s.replace(a, b)
    # byte indices of `pb` in encode: i14 .. i(13+N), then original_len + tier1 / + 1
    s = re.sub(r'\bi23(?![0-9])', f'i{13+N+2}', s)
    s = re.sub(r'\bi22(?![0-9])', f'i{13+N+1}', s)
    old = 'rw [i14_post, i15_post, i16_post, i17_post, i18_post, i19_post, i20_post, i21_post,'
    assert old in s
    s = s.replace(old, 'rw [' + ', '.join(f'i{k}_post' for k in range(14, 14+N)) + ',')
    # names
    s = s.replace('bijou64', 'bijou' + str(B)).replace('BijouAeneas.U64', 'BijouAeneas.' + U)
    s = s.replace('#u64', '#' + u).replace('u64.', u + '.').replace('U64', U).replace('`u64`', f'`{u}`')
    s = s.replace('`bijoux::u64`', f'`bijoux::{u}`')
    s = s.replace('h64', 'hW')
    return s

def arm(w, P, t, last):
    N, T, B = P['N'], P['T'], P['bits']
    U, u = w.upper(), w
    names = P['names'][:t]; addv = P['add'](t); tag = T + t - 1
    posts = ', '.join(n + '_post' for n in names)
    vals = ', '.join(n + '.val' for n in names)
    lines = []
    ind = '      '
    if not last:
        lines += [f'{ind}· step*',
                  f'{ind}  · rename_i hn',
                  f'{ind}    rw [hn] at o1_post',
                  f'{ind}    simp only at o1_post',
                  f'{ind}    rw [o_post, List.map_cons, model_decode_multi _ _ (by decide)]',
                  f'{ind}    have htv : (↑({tag}#8#uscalar : Std.U8) : ℕ) = {tag} := rfl',
                  f'{ind}    simp [toModel, o1_post, htv]',
                  f'{ind}  all_goals']
        b = ind + '    '
        pre = [f'{b}have htv : (↑({tag}#8#uscalar : Std.U8) : ℕ) = {tag} := rfl']
        decide = 'by decide'
    else:
        xs = [f'x{k}' for k in range(T, 255)]
        lines += [f'{ind}· rename_i ' + ' '.join(xs),
                  f'{ind}  have htt : {u}.TAG_THRESHOLD.val = {T} := by unfold {u}.TAG_THRESHOLD; rfl']
        for k in range(T, 255):
            lines += [f'{ind}  have hn{k} : tag.val ≠ {k} := fun hv =>',
                      f'{ind}    x{k} (UScalar.eq_of_val_eq (by rw [hv]; rfl))']
        lines += [f"{ind}  have hge' : {T} ≤ tag.val := by scalar_tac",
                  f"{ind}  have hlt' : tag.val < 256 := by have := tag.hBounds; simpa using this",
                  f'{ind}  have htv : tag.val = 255 := by omega',
                  f'{ind}  step*',
                  f'{ind}  · rename_i hn',
                  f'{ind}    rw [hn] at o1_post',
                  f'{ind}    simp only at o1_post',
                  f'{ind}    rw [o_post, List.map_cons, model_decode_multi _ _ (by omega)]',
                  f'{ind}    simp [toModel, o1_post, htv]',
                  f'{ind}  all_goals']
        b = ind + '    '
        pre = []
        decide = 'by omega'
    lines += [f'{b}rw [‹o1 = some a›] at o1_post',
              f'{b}simp only at o1_post',
              f'{b}obtain ⟨hlen, ha⟩ := o1_post'] + pre + [
              f'{b}have hoff : offset.val = Bijou.bijou{B}.offset {t} := by',
              f'{b}  have := a2_post {t} (by omega)',
              f'{b}  rw [offset_post, ← getElem!_pos, this]',
              f'{b}have hpay : payload.val = Bijou.fromBe ((rest.val.take {t}).map (·.val)) := by',
              f'{b}  have hb := congrArg BitVec.toNat payload_post',
              f'{b}  rw [BitVec.toNat_cast, fromBEBytes_toNat, List.map_map] at hb',
              f'{b}  change payload.val = _ at hb',
              f'{b}  rw [hb]',
              f'{b}  have hlist : a.val = [{", ".join(names)}] := by',
              f'{b}    apply List.ext_getElem',
              f'{b}    · simp',
              f'{b}    · intro idx hidx _',
              f'{b}      have hj : idx < {t} := by have := a.property; simp_all',
              f'{b}      interval_cases idx' + (' <;>' if t > 1 else ''),
              f'{b}      simp [{posts}]']
    if t < N:
        lines += [f'{b}  have hz := fromBe_zeros {N-t} [{vals}]',
                  f'{b}  simp only [List.replicate_succ, List.replicate_zero, List.cons_append, List.nil_append] at hz',
                  f'{b}  rw [← ha, hlist]',
                  f'{b}  simpa [Array.make, Array.from_val, Function.comp] using hz']
    else:
        lines += [f'{b}  rw [← ha, hlist]',
                  f'{b}  simp [Array.make, Array.from_val, Function.comp]']
    lines += [f'{b}rw [o_post, List.map_cons, model_decode_multi _ _ ({decide}), htv,',
              f'{b}  ite_eq_left (by simpa using hlen), ← List.map_take, ← hoff, ← hpay]',
              f'{b}have hmax : {U}.max = 2 ^ {B} - 1 := by rw [{U}.max_eq]; norm_num',
              f'{b}first',
              f'{b}| rw [‹{addv} = none›] at {addv}_post',
              f'{b}  simp only at {addv}_post',
              f'{b}  rw [ite_eq_right (by omega)]',
              f'{b}  rfl',
              f'{b}| rw [‹{addv} = some value›] at {addv}_post',
              f'{b}  obtain ⟨hle, hval, _⟩ := {addv}_post',
              f'{b}  rw [ite_eq_left (by omega)]',
              f'{b}  simp [toModel, hval]']
    return '\n'.join(lines)

def part_b(w, P):
    N, T, B = P['N'], P['T'], P['bits']; U, u = w.upper(), w
    head = f'''-- Generated by scripts/gen_widths.py from its `decode` template, for every
-- width (one arm per tier). Edit the template, not this proof.
theorem decode_spec (buf : Slice Std.U8) :
    {u}.decode buf ⦃ r => toModel r = Bijou.bijou{B}.decode (buf.val.map (·.val)) ⦄ := by
  unfold {u}.decode
  step*
  · rename_i h
    rw [h] at o_post
    simp only at o_post
    simp [o_post, toModel, Bijou.Family.decode]
  · rename_i h
    rw [h] at o_post
    cases p with
    | mk tag rest =>
    simp only at o_post
    change (if tag < {u}.TAG_THRESHOLD then _ else _) ⦃ r =>
      toModel r = Bijou.bijou{B}.decode (buf.val.map (·.val)) ⦄
    split
    · step*
      have hlt' : tag.val < {T} := by
        have : {u}.TAG_THRESHOLD.val = {T} := by unfold {u}.TAG_THRESHOLD; rfl
        scalar_tac
      simp [o_post, toModel, Bijou.Family.decode, show Bijou.bijou{B}.threshold = {T} from rfl,
        hlt', i_post]
    · rename_i hge
      split
'''
    return head + '\n'.join(arm(w, P, t, t == N) for t in range(1, N+1)) + '\n\n'

HEARTBEATS = {'u128': '-- 16 tiers: twice the bytes (and `decode` arms) of `u64`, so these two\n-- whole-function proofs need a larger heartbeat budget than the default.\nset_option maxHeartbeats 1000000 in\n'}
for w, P in W.items():
    if w == 'u64':
        out = pre64 + part_b(w, P) + post64
        open(f'{SRC}/U64.lean', 'w').write(out)
        print(w, len(out.splitlines()))
        continue
    before, after = part_a(w, P).split(MARK)
    out = before + part_b(w, P) + after
    note = (f"The proofs are `U64.lean`'s, with the width constants and Aeneas's\n"
            f"variable names substituted (`scripts/gen_widths.py`). Change `U64.lean`\n"
            f"first, then regenerate this file rather than editing it by hand.\n\n")
    out = out.replace(f'prescribes for `Bijou.bijou{P["bits"]}`.\n\n', f'prescribes for `Bijou.bijou{P["bits"]}`.\n\n' + note, 1)
    if w == 'u128':
        out = out.replace('set_option maxHeartbeats 400000 in', 'set_option maxHeartbeats 2000000 in')
    if w in HEARTBEATS:
        for decl in ['theorem encode_spec (v', 'theorem decode_spec (buf']:
            out = out.replace(decl, HEARTBEATS[w] + decl, 1)
    open(f'{SRC}/{w.upper()}.lean', 'w').write(out)
    print(w, len(out.splitlines()))

# Corollaries: only constants differ.
cor = open(SRC + '/U64Corollaries.lean').read()
for w, P in W.items():
    if w == 'u64':
        continue
    N, B, U = P['N'], P['bits'], w.upper()
    s = cor.replace('2 ^ 64', f'2 ^ {B}').replace('buf.length + 9', f'buf.length + {N+1}')
    s = s.replace('bijou64', f'bijou{B}').replace('U64', U).replace('u64', w)
    s = s.replace('\n-/\n\nopen Aeneas', '\n\nGenerated from `U64Corollaries.lean` by `scripts/gen_widths.py`: change that\nfile, then regenerate, rather than editing this one.\n-/\n\nopen Aeneas', 1)
    open(f'{SRC}/{U}Corollaries.lean', 'w').write(s)
