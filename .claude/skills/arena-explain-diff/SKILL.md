---
name: arena-explain-diff
description: Create a rich, self-contained interactive HTML explanation of an Arena code change, diff, branch, or pull request, OR of any Arena module/crate/aspect a developer wants to learn and be quizzed on. Use when the user wants the background, intuition, implementation walkthrough, data flow, diagrams, or quiz-based reinforcement for a change or a topic, with the result saved as a dated HTML file that stays out of git.
---

# Arena Explain Diff

Produce a single long-form HTML page that teaches a reader how something in Arena works: either a specific code change, or a module/crate/aspect of the codebase the developer wants reinforced. Investigate the surrounding system before explaining anything: the page should make sense to a beginner while still giving an experienced engineer a concise path to the behavior that matters.

## Two modes

- **Change mode**: the subject is a diff, branch, PR, or set of commits. Explain what changed and why.
- **Topic mode**: the subject is an existing module, crate, or aspect of Arena with no pending change (e.g. "quiz me on arena-oracledb's playbook system" or "explain the lifecycle fault model"). Explain how it currently works and why it's shaped that way.

Pick the mode from what the user asked for. If ambiguous, infer the most likely subject from context and state the assumption on the page.

## Workflow

1. Identify the subject and its scope.
   - Change mode: use the current checkout, diff, branch, PR metadata, or user-supplied files as the source of truth.
   - Topic mode: treat the named crate/module/feature as the subject. Read its public API, its tests (unit and component), and its callers to find the behavior worth teaching.
2. Explore relevant surrounding code, tests, configuration, callers, data models, and documentation. Trace the old and new paths (change mode) or the real end-to-end flow (topic mode) far enough to explain behavior, not merely file-by-file edits. Prefer checked-in examples and tests over speculation.
3. Build a narrative before writing HTML:
   - what problem or constraint motivated the change, or what problem the module/feature solves;
   - how the old system behaved (change mode), or the smallest useful mental model of how it works today (topic mode);
   - the smallest useful mental model of the new/current behavior;
   - how the implementation realizes that model;
   - edge cases, trade-offs, and observable consequences.
4. Start from `template.html` in this skill's directory rather than writing the page from scratch. Copy it to the destination path, then fill in the placeholders (`__TITLE__`, `__SUBTITLE__`, `__BACKGROUND_CONTENT__`, `__INTUITION_CONTENT__`, `__CODE_CONTENT__`, `__QUIZ_INTRO__`) and the `questions` array (replace the `/* QUESTIONS_ARRAY */` marker). The template already provides the full theme-aware CSS, diagram patterns, and the quiz-rendering JavaScript — including automatic answer-slot balancing — so do not regenerate that boilerplate; only write the content. Do not depend on external fonts, CDNs, images, JavaScript packages, or network access. Save the result outside the repository at `/tmp/arena-explain-YYYY-MM-DD-<slug>.html`, using the current date in `YYYY-MM-DD` format. If the user explicitly asks for it to be saved inside the repo instead, keep the `arena-explain-` prefix so it still matches the gitignore rule covering these files (see "Keeping generated pages out of git" below) and never stage or commit it.
5. Validate the artifact before handing it off: confirm it exists, is a complete HTML document, contains no leftover `__PLACEHOLDER__` tokens or the `/* QUESTIONS_ARRAY */` marker, contains no external asset dependencies, has working quiz interactions, and satisfies the code-block and quiz checks below. If practical, open it in a browser or use a local HTML inspection tool to catch layout or JavaScript errors.
6. Audit every question before finalizing (see "Fact-checking the quiz" below). Do not ship a question whose claim you have not re-verified against the actual source.

## Keeping generated pages out of git

Every generated file must be named `arena-explain-YYYY-MM-DD-<slug>.html`. The repo's `.gitignore` has a matching `arena-explain-*.html` rule, so a file with this prefix is ignored by git wherever it lands, including if it's generated inside the repository by mistake or by request. Never use a different prefix for this skill's output, and never `git add`/`git commit` the generated file even though it would be ignored by default.

## Required page structure

Include a clear title, a short summary, and a table of contents linking to these sections in this order:

1. **Background** — Explain only the system needed for the subject. Start with an optional beginner-friendly mental model, then narrow to the exact components, contracts, and prior behavior involved.
2. **Intuition** — Explain the core idea before implementation detail. Use small concrete toy inputs and outputs. Show old vs. new behavior (change mode) or a before/after of "naive expectation vs. actual behavior" (topic mode) when comparison makes it clearer.
3. **Code** — Walk through the relevant code in conceptual groups, ordered by execution or dependency flow rather than arbitrary file order. Include precise file and line references when available, but do not dump the whole diff or the whole file.
4. **Quiz** — Include at least ten medium-difficulty, interactive multiple-choice questions about the subject (more is fine for a large subject; never fewer than ten). Clicking an option must immediately show whether it is correct and explain why, including the relevant behavior or code path. Track and display the score live as the reader goes, and show a final pass/fail verdict once every question has been answered — see "Scoring and pass/fail" below.

Use smooth transitions, plain language, and precise systems-oriented prose. Explain jargon on first use. Use callouts for definitions, invariants, important edge cases, and practical consequences. Keep the page readable on phones with responsive CSS. Do not use top-level tabs; make it one continuous page.

## Diagrams and examples

Use a small, reusable set of HTML/CSS diagram patterns rather than ornamental graphics:

- flow diagrams for requests, data, or control flow;
- before/after panels for changed or contrasted behavior;
- labeled component cards for system boundaries;
- compact tables for mappings, invariants, and toy data.

Never use ASCII diagrams. Build diagrams with semantic HTML elements and CSS. Label arrows and include example values whenever the diagram describes data movement. Add accessible text or a caption so the explanation does not depend on visual inspection alone.

## Quiz quality rules

Treat quiz design as part of the explanation, not decoration. Before emitting the page, inspect the full question set together, not question-by-question in isolation.

- Answer-position balancing and per-question option shuffling are both handled automatically by `template.html` — do not hand-write a slot plan or reorder options yourself. The arrangement is seeded from this page's own title and question count (not a single fixed formula shared across every generated page), so it can't be precomputed from the template alone; the balance guarantee only holds if every question has the same number of options (4 is the template's convention), so keep option count uniform across all questions.
- Keep options comparable in length, grammar, specificity, and confidence. Do not make the correct option conspicuously longer, more qualified, or more technically precise than distractors. Shorten or enrich distractors as needed. This is not a style suggestion: after writing the full question set, explicitly compute the character length of every option's visible text and check whether the correct option is the longest (or shortest) option in more than roughly a fifth of the questions — length correlating with correctness is itself a solvable-without-reading-the-page bug, not a quality nitpick. If the check fails, rewrite the outlying options, don't just eyeball it afterward.
- Make every distractor plausible and tied to a real misunderstanding of the subject. Avoid joke answers, obviously impossible claims, "all/none of the above," and trivia that cannot be inferred from the page.
- Ask about behavior, causality, contracts, edge cases, or trade-offs. Avoid questions whose answer can be guessed from a single copied phrase.
- Keep the correct answer and explanation in the page's JavaScript data or DOM so the interaction works offline. Reveal feedback only after selection. Mark the selected option and explain both the right reasoning and, when useful, the misconception behind the distractors.
- Ensure the UI does not expose the answer through styling before selection, DOM labels, `title` attributes, source ordering, or accessibility text. Accessibility labels should describe the option, not its correctness.

## Fact-checking the quiz

Every question, every "correct" marking, and every per-option explanation is a factual claim about Arena's actual behavior. Treat a hallucinated or stale claim here as a bug, not a style issue — a wrong answer key is worse than no quiz at all.

Before finalizing, for every question in the set:

- Re-read the actual source backing the claim (the file/function/behavior the question is about) one more time, independently of whatever you wrote in the Background/Intuition/Code sections. Don't just trust your own earlier paraphrase.
- Confirm the option marked `correct: true` is the one actually true of the current code, not an older version, a different backend/language path, or a plausible-sounding guess.
- Confirm every distractor is actually false of the current code (not merely "sounds wrong") — a distractor that's accidentally also true makes the question unanswerable.
- For change-mode questions, make sure the claim is about the code as it exists *after* the diff, clearly distinguished from pre-change behavior if both are discussed.
- If you cannot verify a claim against checked-in source (e.g. it depends on an external tool's undocumented behavior), either say so explicitly in the option's explanation or drop the question rather than presenting a guess as fact.
- If the audit turns up a question you can't fully verify or that turns out to be wrong, rewrite or replace it — do not ship it anyway because it "looks plausible."

## HTML and code-block constraints

- Escape user/code-derived text for HTML and JavaScript contexts. Preserve meaningful whitespace in code examples.
- Use `<pre><code>...</code></pre>` for code blocks. The CSS for `pre` must explicitly include `white-space: pre` or `white-space: pre-wrap`; verify every code block in the saved source before delivery.
- The quiz-rendering JavaScript (slot balancing, scoring, feedback) lives in `template.html` and should not be rewritten per page — only the `questions` data array is page-specific. If a page's needs genuinely require changing the engine itself, edit `template.html` directly so future pages benefit too, rather than forking the logic inline.
- Include visible focus states and sufficient color contrast. Do not make correctness depend on color alone.
- Avoid claiming behavior that the inspected source does not support. Distinguish observed facts from reasonable interpretation.

## Final handoff

Return the exact absolute path to the generated HTML file as a clickable local-file link. Briefly state what was inspected, and remind the user the file lives outside git (or, if saved inside the repo, that it's covered by the `arena-explain-*.html` gitignore rule and was not staged).
