# Experiences and tips

## `twaclaw`

Personally, I find [The Mechanics of Proof](https://hrmacbeth.github.io/math2001/) the best one to start with. This book has an accompanying [repository](https://github.com/hrmacbeth/math2001). The repo contains the toolchain and templates needed to complete many exercises. 

This repo is a bit old, as is the pinned tooling. However,  `elan` handles multiple versions without issues.  This is what I did:

```bash
git clone https://github.com/hrmacbeth/math2001 
cd math2001
# change the remote if you want
git mv .vscode/ .vscode_backup # you don't want this old vscode conf

# when you call lake, elan will install the required Lean version
lake exe cache get # download dependencies
lake build
# open with vscode and go!
```

I also created a separate repository where I have been adding tutorials and examples, mainly crafted with Claude Code. In [this interview](https://www.youtube.com/watch?v=KzdYKeAqWhY&t=655s), Leonardo de Moura recommends using AI to write Lean and mentions that Terence Tao actually used this method to become familiar with Lean.


