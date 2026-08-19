# Lean

> [Lean](https://lean-lang.org) is an open-source programming language and proof assistant that enables correct, maintainable, and formally verified code.

Lean is a functional programming language written primarily in Lean and a proof assistant. It  can be used to prove mathematical statements, such as theorems, as well as  to verify formal systems, such a software or hardware,  provided they can be represented in Lean.

Lean is based on  [dependent type](https://en.wikipedia.org/wiki/Dependent_type) theory.

## Where to start?

I think this is a tricky one because it has a steeper learning curve than other programming languages, at least in my experience.

Some of you and many others recommend starting with the [Natural Numbers Game](https://adam.math.hhu.de/#/g/leanprover-community/nng4).  The [official documentation](https://lean-lang.org/learn/) includes several books. Which one should you read first? That depends on how you learn. I documented my experiences [here](./experiences.md). Please do the same!

## How to install Lean?

See the [installation notes](https://lean-lang.org/install/). The VSCode plugin is quite nice. However, there are alternatives for Emacs and Vim. Lean comes with a version manager:  `elan`,  and a project manager: `lake` (`make` for Lean):

```bash
lake new my_project
cd my_project
lake build
lake --help
```




## References

- [A recent interview with Leonardo de Moura](https://www.youtube.com/watch?v=KzdYKeAqWhY&t=655s), the introvert creator of Lean.
- [A book describing the origins of Lean](https://www.quantabooks.org/books/the-proof-in-the-code/)
- [Proof and the Art of Mathematics](https://mitpress.mit.edu/9780262539791/proof-and-the-art-of-mathematics/)