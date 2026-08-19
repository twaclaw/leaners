import Leaners
/-!
Differential harness. The refinement proofs in `Leaners/Refine.lean` tie the
specs to the extracted model; this executable ties the whole Lean side to the
Rust binary that actually ships, by diffing outputs on shared vectors. It
guards the parts no theorem covers: charon, aeneas, rustc, and the toolchain
pins.
-/

open Aeneas.Std Leaners.Spec

/-- Byte of the input file to a model byte. `UInt8` is what `String.toUTF8`
gives; `U8` is what the extracted model computes over. -/
def toU8 (b : UInt8) : U8 :=
  UScalar.ofNatCore b.toNat (by have := b.toNat_lt; simp [UScalarTy.numBits]; omega)

def hexDigit (n : Nat) : Char :=
  if n < 10 then Char.ofNat (0x30 + n) else Char.ofNat (0x61 + n - 10)

def hexByte (b : U8) : String :=
  String.singleton (hexDigit (b.val / 16)) ++ String.singleton (hexDigit (b.val % 16))

def hex (bs : List U8) : String := (bs.map hexByte).foldl (· ++ ·) ""

/-- Mirrors Rust's `str::lines` exactly: split on `\n`, drop only the one empty
piece a trailing newline leaves, and strip a trailing `\r` from each line. -/
def splitLines (text : String) : List String :=
  let parts := text.splitOn "\n"
  let parts := match parts.getLast? with
    | some "" => parts.dropLast
    | _ => parts
  parts.map fun l => if l.endsWith "\r" then l.dropRight 1 else l

def main (args : List String) : IO Unit := do
  let path := args.headD "../verified/tests/vectors.txt"
  let text ← IO.FS.readFile path
  for line in splitLines text do
    let v := line.toUTF8.toList.map toU8
    IO.println s!"E {hex (escape v)}"
    IO.println s!"S {hex (slugify v)}"
    IO.println s!"U {if isSafeUrl v then 1 else 0}"
