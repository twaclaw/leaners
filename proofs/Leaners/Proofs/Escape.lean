import Leaners.Spec
import Leaners.Refine
/-!
# Ladder steps 0 and 1

Proved of the spec, then transferred to the extracted model through
`Refine.escape_spec`. The final theorems (`extracted_*`) are statements about
the functions Aeneas extracted from the code that ships.

Step 1 is the property that carries the weight: step 0 alone is satisfied by a
function returning the empty list.
-/

namespace Leaners.Proofs

open Aeneas Aeneas.Std
open Leaners Leaners.Spec

/-! ## Step 0: escaping never emits a delimiter -/

theorem escapeByte_no_delims (b : U8) :
    lt ∉ escapeByte b ∧ gt ∉ escapeByte b ∧ quot ∉ escapeByte b ∧ apos ∉ escapeByte b := by
  unfold escapeByte
  split_ifs with h1 h2 h3 h4 h5
  · subst h1; decide
  · subst h2; decide
  · subst h3; decide
  · subst h4; decide
  · subst h5; decide
  · simp only [List.mem_singleton]
    exact ⟨fun h => h2 h.symm, fun h => h3 h.symm, fun h => h4 h.symm, fun h => h5 h.symm⟩

/-- Step 0, lifted to the whole output. This is the anti-injection core: no `<`
in the escaped text, so nothing in a document can open a tag. -/
theorem escape_no_lt : ∀ s : List U8, lt ∉ escape s
  | [] => by simp [escape]
  | b :: rest => by
      simp only [escape, List.mem_append, not_or]
      exact ⟨(escapeByte_no_delims b).1, escape_no_lt rest⟩

theorem escape_no_gt : ∀ s : List U8, gt ∉ escape s
  | [] => by simp [escape]
  | b :: rest => by
      simp only [escape, List.mem_append, not_or]
      exact ⟨(escapeByte_no_delims b).2.1, escape_no_gt rest⟩

theorem escape_no_quot : ∀ s : List U8, quot ∉ escape s
  | [] => by simp [escape]
  | b :: rest => by
      simp only [escape, List.mem_append, not_or]
      exact ⟨(escapeByte_no_delims b).2.2.1, escape_no_quot rest⟩

theorem escape_no_apos : ∀ s : List U8, apos ∉ escape s
  | [] => by simp [escape]
  | b :: rest => by
      simp only [escape, List.mem_append, not_or]
      exact ⟨(escapeByte_no_delims b).2.2.2, escape_no_apos rest⟩

/-! ## Step 1: the round trip -/

/-- One escaped byte is recovered, whatever follows it. -/
theorem unescapeN_escapeByte (b : U8) (ts : List Nat) :
    unescapeN ((escapeByte b).map (·.val) ++ ts) = b.val :: unescapeN ts := by
  by_cases h1 : b = amp
  · subst h1; simp +decide [escapeByte, unescapeN, amp]
  · by_cases h2 : b = lt
    · subst h2; simp +decide [escapeByte, unescapeN, amp, lt]
    · by_cases h3 : b = gt
      · subst h3; simp +decide [escapeByte, unescapeN, amp, gt]
      · by_cases h4 : b = quot
        · subst h4; simp +decide [escapeByte, unescapeN, amp, quot]
        · by_cases h5 : b = apos
          · subst h5; simp +decide [escapeByte, unescapeN, amp, apos]
          · have hb : escapeByte b = [b] := by
              simp [escapeByte, h1, h2, h3, h4, h5]
            have hval : ¬ b.val = 38 := by
              intro h
              apply h1
              have hamp : amp.val = 38 := by simp [amp]
              scalar_tac
            rw [hb, unescapeN.eq_def]
            simp [hval]

/-- **Ladder step 1.** Escaping loses nothing: unescaping the output's byte
values recovers the input's byte values exactly. Stated on values because
`U8.val` is injective and `Nat` is where list patterns can be matched. -/
theorem escape_roundTrip : ∀ s : List U8,
    unescapeN ((escape s).map (·.val)) = s.map (·.val)
  | [] => by simp [escape, unescapeN]
  | b :: rest => by
      simp only [escape, List.map_append, unescapeN_escapeByte, escape_roundTrip rest,
                 List.map_cons]

/-! ## The level 5 statements: the same, of the extracted model -/

/-- **Steps 0 and 1 for the shipped code.** What the extracted `escape` appends
to the output vector contains no delimiter byte and unescapes back to the
input. The `hcap` hypothesis is the capacity bound under which the Rust does
not abort. -/
theorem extracted_escape_safe (input out : alloc.vec.Vec U8)
    (hcap : out.val.length + 6 * input.val.length ≤ Usize.max) :
    leaners_render.escape.escape input out ⦃ out1 =>
      out1.val = out.val ++ escape input.val ∧
      lt ∉ escape input.val ∧ gt ∉ escape input.val ∧
      quot ∉ escape input.val ∧ apos ∉ escape input.val ∧
      unescapeN ((escape input.val).map (·.val)) = input.val.map (·.val) ⦄ := by
  apply WP.spec_mono (Refine.escape_spec input out hcap)
  intro o ho
  exact ⟨ho, escape_no_lt _, escape_no_gt _, escape_no_quot _, escape_no_apos _,
         escape_roundTrip _⟩

end Leaners.Proofs
