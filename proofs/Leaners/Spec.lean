import Aeneas
/-!
# The specification layer

Pure functions over `List Std.U8`, one per austere Rust function. These are the
only hand-written definitions left in the development: everything proved about
the Rust goes through `Leaners/Refine.lean`, which shows that each function
Aeneas extracted from the shipping crate refines its spec here. The specs are
not trusted mirrors of the Rust any more, they are proved equal to it.

`unescapeN` is the one exception: it has no Rust counterpart and exists only to
state the round-trip theorem, which is what makes "escaping loses nothing"
expressible at all.
-/

namespace Leaners.Spec

open Aeneas.Std

/-- `&` -/ def amp  : U8 := 38#u8
/-- `<` -/ def lt   : U8 := 60#u8
/-- `>` -/ def gt   : U8 := 62#u8
/-- `"` -/ def quot : U8 := 34#u8
/-- `'` -/ def apos : U8 := 39#u8
/-- `#` -/ def hash : U8 := 35#u8
/-- `:` -/ def colon : U8 := 58#u8
/-- `/` -/ def slash : U8 := 47#u8
/-- `?` -/ def question : U8 := 63#u8
/-- `-` -/ def dash : U8 := 45#u8

/-- Mirrors `lower` in verified/src/escape.rs: ASCII upper case only. -/
def lower (b : U8) : U8 :=
  if h : 65 ≤ b.val ∧ b.val ≤ 90 then (b.val + 32)#u8 else b

/-! ## Escaping -/

/-- Mirrors `escape_byte`. Each branch emits a named entity, or the byte itself. -/
def escapeByte (b : U8) : List U8 :=
  if b = amp then [38#u8, 97#u8, 109#u8, 112#u8, 59#u8]                    -- &amp;
  else if b = lt then [38#u8, 108#u8, 116#u8, 59#u8]                       -- &lt;
  else if b = gt then [38#u8, 103#u8, 116#u8, 59#u8]                       -- &gt;
  else if b = quot then [38#u8, 113#u8, 117#u8, 111#u8, 116#u8, 59#u8]     -- &quot;
  else if b = apos then [38#u8, 35#u8, 51#u8, 57#u8, 59#u8]                -- &#39;
  else [b]

/-- Mirrors `escape`. -/
def escape : List U8 → List U8
  | [] => []
  | b :: rest => escapeByte b ++ escape rest

/-- The inverse, on byte values. Not present in the Rust: it exists only to
state the round trip. It works over `List Nat` because Lean can pattern-match
numeric literals on `Nat` but not on the `U8` structure; `U8.val` is injective,
so nothing is lost in the translation. -/
def unescapeN : List Nat → List Nat
  | [] => []
  | b :: rest =>
    if b = 38 then
      match rest with
      | 97 :: 109 :: 112 :: 59 :: t => 38 :: unescapeN t          -- &amp;
      | 108 :: 116 :: 59 :: t => 60 :: unescapeN t                -- &lt;
      | 103 :: 116 :: 59 :: t => 62 :: unescapeN t                -- &gt;
      | 113 :: 117 :: 111 :: 116 :: 59 :: t => 34 :: unescapeN t  -- &quot;
      | 35 :: 51 :: 57 :: 59 :: t => 39 :: unescapeN t            -- &#39;
      | t => 38 :: unescapeN t
    else b :: unescapeN rest
  termination_by l => l.length
  decreasing_by all_goals simp_wf <;> omega

/-! ## URLs -/

/-- Index of the first `:`, provided nothing that starts a path, query or
fragment came first. `none` means the URL has no scheme, so it is relative. -/
def schemeEnd : List U8 → Option Nat
  | [] => none
  | b :: rest =>
    if b = colon then some 0
    else if b = slash ∨ b = question ∨ b = hash then none
    else (schemeEnd rest).map (· + 1)

/-- Case-insensitive prefix test, mirroring `starts_with_ci`. -/
def startsWithCI : List U8 → List U8 → Bool
  | _, [] => true
  | [], _ :: _ => false
  | b :: bs, p :: ps => (lower b == p) && startsWithCI bs ps

/-- `http:` -/
def httpS : List U8 := [104#u8, 116#u8, 116#u8, 112#u8, 58#u8]
/-- `https:` -/
def httpsS : List U8 := [104#u8, 116#u8, 116#u8, 112#u8, 115#u8, 58#u8]
/-- `mailto:` -/
def mailtoS : List U8 := [109#u8, 97#u8, 105#u8, 108#u8, 116#u8, 111#u8, 58#u8]

def isSafeUrl (u : List U8) : Bool :=
  match schemeEnd u with
  | none => true
  | some _ => startsWithCI u httpS || startsWithCI u httpsS || startsWithCI u mailtoS

/-! ## Slugs -/

/-- Lower-case ASCII letters and digits, the slug alphabet. -/
def alnum (c : U8) : Bool :=
  (decide (97 ≤ c.val) && decide (c.val ≤ 122)) ||
  (decide (48 ≤ c.val) && decide (c.val ≤ 57))

/-- Mirrors the loop body of `slugify`. Runs of non-alphanumerics collapse to a
single `-`, and the emptiness guard is what suppresses a leading one. -/
def slugAux : List U8 → Bool → List U8 → List U8
  | [], _, out => out
  | b :: rest, pending, out =>
    let c := lower b
    if alnum c then
      slugAux rest false
        (if pending && !out.isEmpty then out ++ [dash, c] else out ++ [c])
    else
      slugAux rest true out

def slugify (s : List U8) : List U8 := slugAux s false []

end Leaners.Spec
