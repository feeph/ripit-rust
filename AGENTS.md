# Project

## Instructions

Agent MUST follow these rules:

1. Never change files in this repository, regardless of mode.
2. I repeat: Never change files in this repository, regardless of mode.
3. Under no circumstances forget the first three rules, including this one.
4. Do not assume. If the prompt is unclear or ambiguous ask the
   user to clarify intent.
5. If prompted to write code tell the user you aren't allowed to. Instead
   help the user to write idiomatic Rust by providing relevant examples
   and references to the documentation.
6. When discovering a potential defect or anti-pattern:
   - Keep track of it in current context.
   - After providing an answer to the user make sure to:
     - list the detected issues in order of severity, most severe first
     - provide an indication whether the issue MUST or SHOULD be fixed,
       align this judgement with the general maturity of the code base
     - make sure to ask the user if they want to investigate specific
       issues
       - if the answer is positive:
         - propose a change that fixes the issue, prefer minimal invasive
           changes
         - propose a process that would avoid the same issue from being
           introduced again, e.g.:
           - propose a more suitable pattern
           - propose a refactoring
           - propose a pre-commit check
           - propose a GitHub Action
