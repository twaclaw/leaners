import Leaners.Spec
import Leaners.Refine
/-!
# Ladder step 4

The allowlist theorem is the general statement: anything `isSafeUrl` accepts is
either relative or carries one of exactly three schemes. The concrete
rejections are the attack cases, and they hold for an arbitrary tail because
the scheme is decided by the bytes before the colon. Everything is stated of
the spec first, then transferred to the extracted `is_safe_url` through
`Refine.is_safe_url_spec`.
-/

namespace Leaners.Proofs

open Aeneas Aeneas.Std
open Leaners Leaners.Spec

/-- **Ladder step 4.** Nothing else is ever accepted. -/
theorem isSafeUrl_allowlist (u : List U8) (h : isSafeUrl u = true) :
    schemeEnd u = none ∨ startsWithCI u httpS = true ∨
    startsWithCI u httpsS = true ∨ startsWithCI u mailtoS = true := by
  unfold isSafeUrl at h
  split at h
  · exact Or.inl (by assumption)
  · simp only [Bool.or_eq_true] at h
    rcases h with (h | h) | h
    · exact Or.inr (Or.inl h)
    · exact Or.inr (Or.inr (Or.inl h))
    · exact Or.inr (Or.inr (Or.inr h))

/-- `javascript:` in lower case, whatever follows it. -/
theorem rejects_javascript (rest : List U8) :
    isSafeUrl ([106#u8, 97#u8, 118#u8, 97#u8, 115#u8, 99#u8, 114#u8, 105#u8,
                112#u8, 116#u8, 58#u8] ++ rest) = false := by
  simp +decide [Spec.isSafeUrl, Spec.schemeEnd, Spec.startsWithCI, Spec.httpS,
                Spec.httpsS, Spec.mailtoS, Spec.colon, Spec.slash, Spec.question,
                Spec.hash, Spec.lower]

/-- The same with the casing varied. -/
theorem rejects_javascript_mixed_case (rest : List U8) :
    isSafeUrl ([74#u8, 97#u8, 86#u8, 97#u8, 83#u8, 99#u8, 82#u8, 105#u8,
                80#u8, 116#u8, 58#u8] ++ rest) = false := by
  simp +decide [Spec.isSafeUrl, Spec.schemeEnd, Spec.startsWithCI, Spec.httpS,
                Spec.httpsS, Spec.mailtoS, Spec.colon, Spec.slash, Spec.question,
                Spec.hash, Spec.lower]

/-- `data:` URLs, which can carry base64 HTML. -/
theorem rejects_data (rest : List U8) :
    isSafeUrl ([100#u8, 97#u8, 116#u8, 97#u8, 58#u8] ++ rest) = false := by
  simp +decide [Spec.isSafeUrl, Spec.schemeEnd, Spec.startsWithCI, Spec.httpS,
                Spec.httpsS, Spec.mailtoS, Spec.colon, Spec.slash, Spec.question,
                Spec.hash, Spec.lower]

/-- A relative URL is always accepted: no colon before the first `/`. -/
theorem accepts_relative (rest : List U8) :
    isSafeUrl ([110#u8, 111#u8, 116#u8, 101#u8, 115#u8, 47#u8] ++ rest) = true := by
  simp +decide [Spec.isSafeUrl, Spec.schemeEnd, Spec.startsWithCI, Spec.colon,
                Spec.slash, Spec.question, Spec.hash]

/-! ## The level 5 statements: the same, of the extracted model -/

/-- **Step 4 for the shipped code.** Whatever the extracted `is_safe_url`
accepts is relative or carries one of exactly three schemes. -/
theorem extracted_is_safe_url_allowlist (url : alloc.vec.Vec U8) :
    leaners_render.escape.is_safe_url url ⦃ b =>
      b = true → schemeEnd url.val = none ∨ startsWithCI url.val httpS = true ∨
        startsWithCI url.val httpsS = true ∨ startsWithCI url.val mailtoS = true ⦄ := by
  apply WP.spec_mono (Refine.is_safe_url_spec url)
  intro b hb hbtrue
  rw [hb] at hbtrue
  exact isSafeUrl_allowlist url.val hbtrue

/-- The attack case, of the shipped code: a `javascript:` URL is rejected. -/
theorem extracted_rejects_javascript (url : alloc.vec.Vec U8) (rest : List U8)
    (h : url.val = [106#u8, 97#u8, 118#u8, 97#u8, 115#u8, 99#u8, 114#u8, 105#u8,
                    112#u8, 116#u8, 58#u8] ++ rest) :
    leaners_render.escape.is_safe_url url ⦃ b => b = false ⦄ := by
  apply WP.spec_mono (Refine.is_safe_url_spec url)
  intro b hb
  rw [hb, h, rejects_javascript]

end Leaners.Proofs
