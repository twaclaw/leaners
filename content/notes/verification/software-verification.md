# Software verification with Lean

>  Program testing can be used to show the presence of bugs, but never to show their absence!
>  Edsger W. Dijkstra 


In Lean, verification means stating what a program should do as a mathematical proposition and then creating a proof of it that a machine can check. 

Lean is a programming language and proof assistant based on dependent type theory. In this theory, a proof is a term whose type is the proposition, and every term is validated by a small trusted kernel.

The practical difference from testing is quantification: a test samples a finite number of inputs, whereas a proof covers every possible input, including those that have never been tried. 

A combination that seems to work well is Rust and Lean. There are tools that can mechanically translate Rust into Lean and obtain a faithful Lean model of the Rust code; [Aeneas](https://lean-lang.org/use-cases/aeneas/) is one such tool.


## Examples

- [Lean-zip](https://github.com/kim-em/lean-zip). A Lean model that verifies `decompress(compress(x))=x` and uses AI to optimize the function based on that harness.

```lean
/-- Decompressing the output of `compress` returns the original data,
    for every input and every compression level. -/
theorem zlib_decompressSingle_compress (data : ByteArray) (level : UInt8)
    (maxOutputSize : Nat) (hsize : data.size ≤ maxOutputSize) :
    ZlibDecode.decompressSingle (ZlibEncode.compress data level) maxOutputSize = .ok data
```
- [AWS post on verification with Lean](https://aws.amazon.com/blogs/opensource/lean-into-verified-software-development/)
- This may be a contrived example, but this very document practices what it preaches and implements a verification for one part of the parser. See [About this site](../about-this-site.md) for details.
