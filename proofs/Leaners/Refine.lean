import Extracted.LeanersRender
import Leaners.Spec
/-!
# Refinement: the extracted model computes the specs

One theorem per function that Aeneas extracted from the shipping crate, each
stating that the monadic, `Result`-typed extraction returns exactly what the
corresponding pure spec in `Leaners/Spec.lean` computes. These proofs are what
turns the specs from trusted mirrors of the Rust into consequences of it: every
ladder theorem proved about a spec transfers to the extracted model through the
theorem here.

The `≤ Usize.max` hypotheses are real, not bureaucratic: `Vec::push` in Rust
aborts on capacity overflow, so the extraction fails (and the refinement could
not hold) on inputs whose escaped or slugified form would not fit in a vector.

Proof style follows the Aeneas tutorial: `unfold`, `step`, recursion on the
theorem itself justified by `termination_by`/`scalar_decr_tac`.
-/

namespace Leaners.Refine

open Aeneas Aeneas.Std Result
open leaners_render
open Leaners

/-! ## escape.rs -/

@[step]
theorem lower_spec (b : U8) :
    escape.lower b ⦃ r => r = Spec.lower b ⦄ := by
  unfold escape.lower
  split
  · split
    · step as ⟨ r, hr ⟩
      have hc : 65 ≤ b.val ∧ b.val ≤ 90 := by scalar_tac
      simp [Spec.lower, hc]
      scalar_tac
    · simp [Spec.lower]
      scalar_tac
  · simp [Spec.lower]
    scalar_tac

@[step]
theorem push_all_loop_spec (out : alloc.vec.Vec U8) (s : Slice U8) (i : Usize)
    (hcap : out.val.length + (s.val.length - i.val) ≤ Usize.max) :
    escape.push_all_loop out s i
      ⦃ out1 => out1.val = out.val ++ s.val.drop i.val ⦄ := by
  unfold escape.push_all_loop
  simp
  split
  · step as ⟨ b, hb ⟩
    step as ⟨ out1, hout1 ⟩
    step as ⟨ i1, hi1 ⟩
    have IH := push_all_loop_spec out1 s i1 (by scalar_tac)
    step with IH as ⟨ res, hres ⟩
    clear IH
    simp only [hres, hout1, hi1, hb]
    rw [List.drop_eq_getElem_cons (l := s.val) (i := i.val) (by scalar_tac)]
    simp
  · have : s.val.length ≤ i.val := by scalar_tac
    simp_all
termination_by s.val.length - i.val
decreasing_by scalar_decr_tac

@[step]
theorem push_all_spec (out : alloc.vec.Vec U8) (s : Slice U8)
    (hcap : out.val.length + s.val.length ≤ Usize.max) :
    escape.push_all out s ⦃ out1 => out1.val = out.val ++ s.val ⦄ := by
  unfold escape.push_all
  step as ⟨ out1, h ⟩
  simp_all

theorem escapeByte_length_le (b : U8) : (Spec.escapeByte b).length ≤ 6 := by
  unfold Spec.escapeByte
  split_ifs <;> simp

@[step]
theorem escape_byte_spec (b : U8) (out : alloc.vec.Vec U8)
    (hcap : out.val.length + 6 ≤ Usize.max) :
    escape.escape_byte b out
      ⦃ out1 => out1.val = out.val ++ Spec.escapeByte b ⦄ := by
  unfold escape.escape_byte
  simp only [Spec.escapeByte, Spec.amp, Spec.lt, Spec.gt, Spec.quot, Spec.apos, lift, bind_ok]
  split
  · step as ⟨ out1, h1 ⟩
    simp_all [Array.to_slice, Array.make]
  · split
    · step as ⟨ out1, h1 ⟩
      simp_all [Array.to_slice, Array.make]
    · split
      · step as ⟨ out1, h1 ⟩
        simp_all [Array.to_slice, Array.make]
      · split
        · step as ⟨ out1, h1 ⟩
          simp_all [Array.to_slice, Array.make]
        · split
          · step as ⟨ out1, h1 ⟩
            simp_all [Array.to_slice, Array.make]
          · step as ⟨ out1, h1 ⟩
            simp_all

@[step]
theorem escape_loop_spec (input out : alloc.vec.Vec U8) (i : Usize)
    (hcap : out.val.length + 6 * (input.val.length - i.val) ≤ Usize.max) :
    escape.escape_loop input out i
      ⦃ out1 => out1.val = out.val ++ Spec.escape (input.val.drop i.val) ⦄ := by
  unfold escape.escape_loop
  simp
  split
  · step as ⟨ b, hb ⟩
    step as ⟨ out1, hout1 ⟩
    step as ⟨ i1, hi1 ⟩
    have hle := escapeByte_length_le b
    have IH := escape_loop_spec input out1 i1 (by scalar_tac)
    step with IH as ⟨ res, hres ⟩
    clear IH
    simp_all
    rw [List.drop_eq_getElem_cons (l := input.val) (i := i.val) (by scalar_tac)]
    simp [Spec.escape]
  · have : input.val.length ≤ i.val := by scalar_tac
    simp_all [Spec.escape]
termination_by input.val.length - i.val
decreasing_by scalar_decr_tac

/-- The extracted `escape` writes exactly the spec's escaping of the input
after whatever the output vector already held. -/
theorem escape_spec (input out : alloc.vec.Vec U8)
    (hcap : out.val.length + 6 * input.val.length ≤ Usize.max) :
    escape.escape input out
      ⦃ out1 => out1.val = out.val ++ Spec.escape input.val ⦄ := by
  unfold escape.escape
  step as ⟨ out1, h ⟩
  simp_all

/-! ## escape.rs, the URL allowlist -/

theorem startsWithCI_false_of_short (u p : List U8) (h : u.length < p.length) :
    Spec.startsWithCI u p = false := by
  induction u generalizing p with
  | nil =>
    cases p with
    | nil => simp at h
    | cons y ys => simp [Spec.startsWithCI]
  | cons x xs ih =>
    cases p with
    | nil => simp at h
    | cons y ys =>
      simp only [Spec.startsWithCI, Bool.and_eq_false_iff]
      right
      exact ih ys (by simpa using h)

@[step]
theorem starts_with_ci_loop_spec (url : alloc.vec.Vec U8) (p : Slice U8) (i : Usize)
    (hlen : p.val.length ≤ url.val.length) (hi : i.val ≤ p.val.length) :
    escape.starts_with_ci_loop url p i
      ⦃ b => b = Spec.startsWithCI (url.val.drop i.val) (p.val.drop i.val) ⦄ := by
  unfold escape.starts_with_ci_loop
  simp
  split
  · step as ⟨ c, hc ⟩
    step as ⟨ lc, hlc ⟩
    step as ⟨ pc, hpc ⟩
    have hcu := List.drop_eq_getElem_cons (l := url.val) (i := i.val) (by scalar_tac)
    have hcp := List.drop_eq_getElem_cons (l := p.val) (i := i.val) (by scalar_tac)
    -- The initial simp flips `if lc != pc` into `if lc = pc`, so the equal
    -- (continue) branch comes first.
    split
    · step as ⟨ i1, hi1 ⟩
      have IH := starts_with_ci_loop_spec url p i1 hlen (by scalar_tac)
      step with IH as ⟨ res, hres ⟩
      clear IH
      have hbeq : Spec.lower url.val[i.val] = p.val[i.val] := by
        have h2 : lc = pc := by scalar_tac
        simp_all
      rw [hcu, hcp]
      simp_all [Spec.startsWithCI]
    · have hbeq : ¬ Spec.lower url.val[i.val] = p.val[i.val] := by
        have h2 : ¬ lc = pc := by scalar_tac
        simp_all
      rw [hcu, hcp]
      simp_all [Spec.startsWithCI]
  · have : p.val.length ≤ i.val := by scalar_tac
    simp_all [Spec.startsWithCI]
termination_by p.val.length - i.val
decreasing_by scalar_decr_tac

@[step]
theorem starts_with_ci_spec (url : alloc.vec.Vec U8) (p : Slice U8) :
    escape.starts_with_ci url p ⦃ b => b = Spec.startsWithCI url.val p.val ⦄ := by
  unfold escape.starts_with_ci
  simp
  split
  · simp [startsWithCI_false_of_short url.val p.val (by scalar_tac)]
  · have hloop := starts_with_ci_loop_spec url p 0#usize (by scalar_tac) (by scalar_tac)
    step with hloop as ⟨ b, hb ⟩
    simp_all

theorem schemeEnd_lt_length (u : List U8) (k : Nat) (h : Spec.schemeEnd u = some k) :
    k < u.length := by
  induction u generalizing k with
  | nil => simp [Spec.schemeEnd] at h
  | cons b rest ih =>
    unfold Spec.schemeEnd at h
    split at h
    · simp at h ⊢
      omega
    · split at h
      · simp at h
      · cases hse : Spec.schemeEnd rest with
        | none => simp [hse] at h
        | some k' =>
          rw [hse] at h
          simp at h
          have := ih k' hse
          simp
          omega

@[step]
theorem is_safe_url_loop_spec (url : alloc.vec.Vec U8) (i colon : Usize)
    (hi : i.val ≤ url.val.length) :
    escape.is_safe_url_loop url i colon
      ⦃ r => r.val = ((Spec.schemeEnd (url.val.drop i.val)).map (i.val + ·)).getD colon.val ⦄ := by
  unfold escape.is_safe_url_loop
  simp
  split
  · step as ⟨ c, hc ⟩
    have hcons := List.drop_eq_getElem_cons (l := url.val) (i := i.val) (by scalar_tac)
    split
    · -- found the colon at i
      have IH := is_safe_url_loop_spec url (alloc.vec.Vec.len url) i (by scalar_tac)
      step with IH as ⟨ r, hr ⟩
      clear IH
      rw [hcons]
      simp_all [Spec.schemeEnd, Spec.colon]
    · split
      · -- a path started: no scheme
        have IH := is_safe_url_loop_spec url (alloc.vec.Vec.len url) colon (by scalar_tac)
        step with IH as ⟨ r, hr ⟩
        clear IH
        rw [hcons]
        simp_all [Spec.schemeEnd, Spec.colon, Spec.slash, Spec.question, Spec.hash]
      · split
        · -- a query started
          have IH := is_safe_url_loop_spec url (alloc.vec.Vec.len url) colon (by scalar_tac)
          step with IH as ⟨ r, hr ⟩
          clear IH
          rw [hcons]
          simp_all [Spec.schemeEnd, Spec.colon, Spec.slash, Spec.question, Spec.hash]
        · split
          · -- a fragment started
            have IH := is_safe_url_loop_spec url (alloc.vec.Vec.len url) colon (by scalar_tac)
            step with IH as ⟨ r, hr ⟩
            clear IH
            rw [hcons]
            simp_all [Spec.schemeEnd, Spec.colon, Spec.slash, Spec.question, Spec.hash]
          · -- an ordinary byte: keep scanning
            step as ⟨ i1, hi1 ⟩
            have IH := is_safe_url_loop_spec url i1 colon (by scalar_tac)
            step with IH as ⟨ r, hr ⟩
            clear IH
            rw [hcons]
            simp only [Spec.schemeEnd, Spec.colon, Spec.slash, Spec.question, Spec.hash]
            have hb1 : ¬ url.val[i.val] = 58#u8 := by simp_all
            have hb2 : ¬ (url.val[i.val] = 47#u8 ∨ url.val[i.val] = 63#u8 ∨ url.val[i.val] = 35#u8) := by
              simp_all
            simp only [hb1, hb2, if_false]
            cases hse : Spec.schemeEnd (url.val.drop (i.val + 1)) <;> simp_all <;> omega
  · have : url.val.length ≤ i.val := by scalar_tac
    simp_all [Spec.schemeEnd]
termination_by url.val.length - i.val
decreasing_by all_goals first | scalar_decr_tac | (simp_all; scalar_tac)

theorem is_safe_url_spec (url : alloc.vec.Vec U8) :
    escape.is_safe_url url ⦃ b => b = Spec.isSafeUrl url.val ⦄ := by
  unfold escape.is_safe_url
  simp only [lift, bind_ok]
  have hloop := is_safe_url_loop_spec url 0#usize (alloc.vec.Vec.len url) (by scalar_tac)
  step with hloop as ⟨ colon1, hcolon ⟩
  clear hloop
  simp at hcolon
  split
  · rename_i heq
    cases hse : Spec.schemeEnd url.val with
    | none => simp [Spec.isSafeUrl, hse]
    | some k =>
      exfalso
      have hk := schemeEnd_lt_length url.val k hse
      simp [hse] at hcolon
      scalar_tac
  · rename_i hne
    cases hse : Spec.schemeEnd url.val with
    | none =>
      exfalso
      simp [hse] at hcolon
      scalar_tac
    | some k =>
      step as ⟨ b1, hb1 ⟩
      split
      · simp_all [Spec.isSafeUrl, Spec.httpS, Array.to_slice, Array.make]
      · step as ⟨ b2, hb2 ⟩
        split
        · simp_all [Spec.isSafeUrl, Spec.httpS, Spec.httpsS, Array.to_slice, Array.make]
        · step as ⟨ b3, hb3 ⟩
          simp_all [Spec.isSafeUrl, Spec.httpS, Spec.httpsS, Spec.mailtoS,
                    Array.to_slice, Array.make]

/-! ## slug.rs -/

/-- The optional separating dash of `slugify_loop`, stated in the simp-normal
form the proof goals take. -/
theorem dash_spec (out : alloc.vec.Vec U8) (pending : Bool)
    (hcap : out.val.length + 1 ≤ Usize.max) :
    (if pending = true
     then if 0 < out.val.length then alloc.vec.Vec.push out 45#u8 else ok out
     else ok out)
      ⦃ out2 => out2.val = (if pending && !out.val.isEmpty
                            then out.val ++ [Spec.dash]
                            else out.val) ⦄ := by
  split
  · split
    · step as ⟨ out2, hout2 ⟩
      have hne : out.val.isEmpty = false := by
        cases hv : out.val
        · simp_all
        · simp
      simp_all [Spec.dash]
    · have hemp : out.val.isEmpty = true := by
        cases hv : out.val
        · simp
        · simp_all
      simp_all
  · simp_all

/-- The whole emit block: an optional dash, the byte itself, and the cleared
pending flag. Factored out because the extraction duplicates this block once
per alphanumeric range of the character test. -/
theorem emit_spec (out : alloc.vec.Vec U8) (c : U8) (pending : Bool)
    (hcap : out.val.length + 2 ≤ Usize.max) :
    (do
      let out2 ← if pending = true
                 then if 0 < out.val.length then alloc.vec.Vec.push out 45#u8 else ok out
                 else ok out
      let out3 ← alloc.vec.Vec.push out2 c
      ok (out3, false))
      ⦃ out1 pd1 => pd1 = false ∧
          out1.val = (if pending && !out.val.isEmpty
                      then out.val ++ [Spec.dash, c]
                      else out.val ++ [c]) ⦄ := by
  apply WP.spec_bind (dash_spec out pending (by scalar_tac))
  intro out2 hout2
  have hlen2 : out2.val.length ≤ out.val.length + 1 := by
    split at hout2 <;> simp_all <;> omega
  step as ⟨ out3, hout3 ⟩
  cases hpe : (pending && !out.val.isEmpty) <;> simp_all

@[step]
theorem slugify_loop_spec (input out : alloc.vec.Vec U8) (i : Usize) (pending : Bool)
    (hcap : out.val.length + 2 * (input.val.length - i.val) ≤ Usize.max) :
    slug.slugify_loop input out i pending
      ⦃ out1 => out1.val = Spec.slugAux (input.val.drop i.val) pending out.val ⦄ := by
  unfold slug.slugify_loop
  simp
  split
  · step as ⟨ b, hb ⟩
    step as ⟨ c, hc ⟩
    have hcons := List.drop_eq_getElem_cons (l := input.val) (i := i.val) (by scalar_tac)
    split
    · split
      · -- a lower-case letter
        apply WP.spec_bind (emit_spec out c pending (by scalar_tac))
        intro x hx
        obtain ⟨ out1, pd1 ⟩ := x
        simp only [WP.uncurry'_pair] at hx
        obtain ⟨ hpd1, hout1 ⟩ := hx
        show (do
          let i3 ← i + 1#usize
          slug.slugify_loop input out1 i3 pd1)
          ⦃ o => o.val = Spec.slugAux (input.val.drop i.val) pending out.val ⦄
        step as ⟨ i3, hi3 ⟩
        have hlen2 : out1.val.length ≤ out.val.length + 2 := by
          split at hout1 <;> simp_all <;> omega
        have IH := slugify_loop_spec input out1 i3 pd1 (by scalar_tac)
        step with IH as ⟨ res, hres ⟩
        clear IH
        have halnum : Spec.alnum c = true := by simp [Spec.alnum]; scalar_tac
        rw [hcons]
        simp only [hres, hpd1, hi3, Spec.slugAux]
        simp [← hb, ← hc, halnum, hout1]
      · split
        · split
          · -- a digit, reached from the high side of the letter test
            apply WP.spec_bind (emit_spec out c pending (by scalar_tac))
            intro x hx
            obtain ⟨ out1, pd1 ⟩ := x
            simp only [WP.uncurry'_pair] at hx
            obtain ⟨ hpd1, hout1 ⟩ := hx
            show (do
              let i3 ← i + 1#usize
              slug.slugify_loop input out1 i3 pd1)
              ⦃ o => o.val = Spec.slugAux (input.val.drop i.val) pending out.val ⦄
            apply WP.spec_bind (UScalar.add_spec (by scalar_tac))
            intro i3 hi3
            have hlen2 : out1.val.length ≤ out.val.length + 2 := by
              split at hout1 <;> simp_all <;> omega
            apply WP.spec_mono (slugify_loop_spec input out1 i3 pd1 (by scalar_tac))
            intro o ho
            have halnum : Spec.alnum c = true := by simp [Spec.alnum]; scalar_tac
            rw [ho, hcons]
            simp only [hpd1, hi3, Spec.slugAux]
            simp [← hb, ← hc, halnum, hout1]
          · -- not alphanumeric
            step as ⟨ i3, hi3 ⟩
            have IH := slugify_loop_spec input out i3 true (by scalar_tac)
            step with IH as ⟨ res, hres ⟩
            clear IH
            have halnum : Spec.alnum c = false := by simp [Spec.alnum]; scalar_tac
            rw [hcons]
            simp only [hres, hi3, Spec.slugAux]
            simp [← hb, ← hc, halnum]
        · apply WP.spec_bind (UScalar.add_spec (by scalar_tac))
          intro i3 hi3
          apply WP.spec_mono (slugify_loop_spec input out i3 true (by scalar_tac))
          intro o ho
          have halnum : Spec.alnum c = false := by simp [Spec.alnum]; scalar_tac
          rw [ho, hcons]
          simp only [hi3, Spec.slugAux]
          simp [← hb, ← hc, halnum]
    · split
      · split
        · -- a digit, reached from the low side of the letter test
          apply WP.spec_bind (emit_spec out c pending (by scalar_tac))
          intro x hx
          obtain ⟨ out1, pd1 ⟩ := x
          simp only [WP.uncurry'_pair] at hx
          obtain ⟨ hpd1, hout1 ⟩ := hx
          show (do
            let i3 ← i + 1#usize
            slug.slugify_loop input out1 i3 pd1)
            ⦃ o => o.val = Spec.slugAux (input.val.drop i.val) pending out.val ⦄
          apply WP.spec_bind (UScalar.add_spec (by scalar_tac))
          intro i3 hi3
          have hlen2 : out1.val.length ≤ out.val.length + 2 := by
            split at hout1 <;> simp_all <;> omega
          apply WP.spec_mono (slugify_loop_spec input out1 i3 pd1 (by scalar_tac))
          intro o ho
          have halnum : Spec.alnum c = true := by simp [Spec.alnum]; scalar_tac
          rw [ho, hcons]
          simp only [hpd1, hi3, Spec.slugAux]
          simp [← hb, ← hc, halnum, hout1]
        · step as ⟨ i3, hi3 ⟩
          have IH := slugify_loop_spec input out i3 true (by scalar_tac)
          step with IH as ⟨ res, hres ⟩
          clear IH
          have halnum : Spec.alnum c = false := by simp [Spec.alnum]; scalar_tac
          rw [hcons]
          simp only [hres, hi3, Spec.slugAux]
          simp [← hb, ← hc, halnum]
      · step as ⟨ i3, hi3 ⟩
        have IH := slugify_loop_spec input out i3 true (by scalar_tac)
        step with IH as ⟨ res, hres ⟩
        clear IH
        have halnum : Spec.alnum c = false := by simp [Spec.alnum]; scalar_tac
        rw [hcons]
        simp only [hres, hi3, Spec.slugAux]
        simp [← hb, ← hc, halnum]
  · have : input.val.length ≤ i.val := by scalar_tac
    simp_all [Spec.slugAux]
termination_by input.val.length - i.val
decreasing_by all_goals first | scalar_decr_tac | (simp_all; scalar_tac)

/-- The extracted `slugify` computes exactly the spec's slug. -/
theorem slugify_spec (input : alloc.vec.Vec U8)
    (hcap : 2 * input.val.length ≤ Usize.max) :
    slug.slugify input ⦃ out => out.val = Spec.slugify input.val ⦄ := by
  unfold slug.slugify
  have hloop := slugify_loop_spec input (alloc.vec.Vec.new U8) 0#usize false
    (by simp [alloc.vec.Vec.new]; scalar_tac)
  step with hloop as ⟨ out, hout ⟩
  simp_all [Spec.slugify, alloc.vec.Vec.new]

end Leaners.Refine
