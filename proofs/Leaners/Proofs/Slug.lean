import Leaners.Spec
import Leaners.Refine
/-!
# Ladder step 2, the charset invariant

Every byte of a slug is a lower-case alphanumeric or a `-`. Proved with the
accumulator as an invariant, then transferred to the extracted `slugify`
through `Refine.slugify_spec`. Idempotence and step 3's `Nodup` are still
open; `verified/tests/props.rs` covers them empirically in the meantime.
-/

namespace Leaners.Proofs

open Aeneas Aeneas.Std
open Leaners Leaners.Spec

/-- A byte a slug is allowed to contain. -/
def okByte (b : U8) : Prop := alnum b = true ∨ b = dash

theorem slugAux_charset :
    ∀ (s : List U8) (p : Bool) (out : List U8),
      (∀ b ∈ out, okByte b) → ∀ b ∈ slugAux s p out, okByte b
  | [], _, out, h => by simpa [slugAux] using h
  | c :: rest, p, out, h => by
      simp only [slugAux]
      split
      · rename_i hc
        refine slugAux_charset rest false _ ?_
        split
        · intro b hb
          rcases List.mem_append.1 hb with hb | hb
          · exact h b hb
          · rcases List.mem_cons.1 hb with rfl | hb
            · exact Or.inr rfl
            · rcases List.mem_cons.1 hb with rfl | hb
              · exact Or.inl hc
              · simp at hb
        · intro b hb
          rcases List.mem_append.1 hb with hb | hb
          · exact h b hb
          · rcases List.mem_cons.1 hb with rfl | hb
            · exact Or.inl hc
            · simp at hb
      · exact slugAux_charset rest true out h

/-- **Ladder step 2, charset.** -/
theorem slugify_charset (s : List U8) : ∀ b ∈ slugify s, okByte b :=
  slugAux_charset s false [] (by simp)

/-! ## The level 5 statement: the same, of the extracted model -/

/-- **Step 2 for the shipped code.** Every byte the extracted `slugify`
produces is a lower-case alphanumeric or a dash, so an anchor id can never
carry markup or need escaping. -/
theorem extracted_slugify_charset (input : alloc.vec.Vec U8)
    (hcap : 2 * input.val.length ≤ Usize.max) :
    leaners_render.slug.slugify input ⦃ out => ∀ b ∈ out.val, okByte b ⦄ := by
  apply WP.spec_mono (Refine.slugify_spec input hcap)
  intro o ho
  rw [ho]
  exact slugify_charset input.val

end Leaners.Proofs
