# Project Guidelines

## Commit Size

- Keep commits extremely small and narrowly focused.
- Split one logical change into multiple commits when doing so keeps each commit atomic and easy to review.
- Include only the files and lines required for one clear step in each commit.
- Do not mix behavior changes, refactors, formatting, documentation, or tests in the same commit unless they are inseparable.
- Keep commit messages concise and limited to the change introduced by that commit.

## Branch Names

- Use the format `<type>/<short-description>`.
- Use a lowercase change type such as `feat`, `fix`, `chore`, `docs`, `refactor`, or `test`.
- Write the description in lowercase kebab-case and keep it short.
- Do not include assistant, automation, or tool branding in branch names.

## Pull Requests

- Open a pull request autonomously when the requested changes are complete.
- Keep each pull request focused on the smallest coherent set of changes.
- The pull request title and description must accurately summarize the changes and relevant validation.
- Do not wait for an additional confirmation before opening the pull request.

## Code Comments and Inline Documentation

- Write code comments, TODOs, docstrings, and other code-level annotations in English.
- Keep comments concise and explain intent or non-obvious constraints rather than restating the code.

## Naming and Project Text

- Do not include assistant, automation, or tool branding in commit messages, pull request text, documentation, source files, or other project artifacts.
