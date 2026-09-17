# Minimal fixture

This directory is reserved for the first end-to-end schema fixture.

The first fixture should be intentionally smaller than the complete UCI corpus and should exercise:

- one namespace;
- one enum;
- one restricted scalar;
- one record;
- one optional field;
- one bounded repeated field;
- one publishable message classification.

Do not expand the fixture merely to make the parser look feature-complete. Add a schema construct only when there is a corresponding IR representation, backend behavior, and test.
