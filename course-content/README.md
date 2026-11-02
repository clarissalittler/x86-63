# Versioned course lessons

These short programs are the student-facing compatibility fixtures bundled in
both the browser and native releases. They mirror the maintained CS201 Lecture
3 through Lecture 6 sources as reviewed in July 2026. Lecture 4's repeated,
comment-only variant of `firstadd.s` uses the Lecture 3 fixture. The neighboring
CS201 repository remains read-only; future drift checks should report
differences rather than editing it. `factTrace.s` is the one purpose-built lab
harness: it fixes factorial's input at 5 so students can inspect recursion
before adding the real `readInt.s` and `writeInt.s` modules.

`practice/` contains five purpose-built repair labs. Unlike the maintained
lecture fixtures, two intentionally fail to assemble and three run with a
visible semantic or ABI mistake. Their versioned challenge metadata supplies
goals and progressive hints without changing the source automatically.

`assignment2/` contains the course's complete minimal `sum5.s` skeleton. The
lesson manifest links it with the maintained Lecture 6 `readInt.s` and
`writeInt.s` helpers. Its integration test deliberately waits at each of five
reads before submitting 10, 20, 30, 40, and 50; this preserves the assignment's
line-oriented terminal contract and verifies the resulting array and `150\n`
output.
