# Adopting unref

1. Run `uvx unref` in your project and delete what is really unused.
2. For a finding that is not, check that the [frameworks](frameworks.md) you use are covered, set
   [entry points or used names](configuration.md), or [ignore](rules.md#ignoring-a-finding) it.
3. Report what is still wrong or missed in an [issue](https://github.com/Apakottur/unref/issues/new?template=bug_report.yml).
   A small reproduction helps the most, but the findings and the code around them are enough. For a
   public project, a link is enough: we can run unref on it and work through the findings with you.
4. Once it is clean, run unref as a [pre-commit hook or in CI](usage.md) so new dead code is caught.
