use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Lesson {
    pub id: &'static str,
    pub title: &'static str,
    pub lecture: u8,
    pub summary: &'static str,
    pub prediction: &'static str,
    pub module_name: &'static str,
    pub source: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct LessonModule {
    pub module_name: &'static str,
    pub source: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Challenge {
    pub goal: &'static str,
    pub success: &'static str,
    pub hints: &'static [&'static str],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct Assignment {
    pub goal: &'static str,
    pub success: &'static str,
    pub requirements: &'static [&'static str],
    pub sample_inputs: &'static [&'static str],
    pub expected_output: &'static str,
}

#[derive(Serialize)]
pub struct LessonView<'a> {
    #[serde(flatten)]
    pub lesson: &'a Lesson,
    pub support_modules: &'static [LessonModule],
    pub challenge: Option<&'static Challenge>,
    pub collection: &'static str,
    pub assignment: Option<&'static Assignment>,
}

pub fn lessons() -> &'static [Lesson] {
    static LESSONS: [Lesson; 40] = [
        Lesson {
            id: "first",
            title: "The deliberately incomplete program",
            lecture: 3,
            summary: "Load the exit syscall number, then discover that nothing invokes it.",
            prediction: "What will happen after the mov if there is no syscall instruction?",
            module_name: "first.s",
            source: include_str!("../../../course-content/lecture3/first.s"),
        },
        Lesson {
            id: "firstfixed",
            title: "Exit, fixed",
            lecture: 3,
            summary: "Invoke exit and compare the 64-bit argument with the shell's 8-bit status.",
            prediction: "Why will `echo $?` show 255 even though %rdi contains -1?",
            module_name: "firstfixed.s",
            source: include_str!("../../../course-content/lecture3/firstfixed.s"),
        },
        Lesson {
            id: "firstadd",
            title: "Addition and AT&T operand order",
            lecture: 3,
            summary: "Add two register values and return their sum as the exit status.",
            prediction: "After `add %rbx,%rcx`, which register contains 30?",
            module_name: "firstadd.s",
            source: include_str!("../../../course-content/lecture3/firstadd.s"),
        },
        Lesson {
            id: "firstsub",
            title: "Subtraction reads right-to-left",
            lecture: 3,
            summary: "See why `sub source,destination` computes destination minus source.",
            prediction: "Will `%rcx` become 10 or -10? Explain before stepping.",
            module_name: "firstsub.s",
            source: include_str!("../../../course-content/lecture3/firstsub.s"),
        },
        Lesson {
            id: "addglobal",
            title: "A global value at an absolute address",
            lecture: 4,
            summary: "Load, update, and store one .quad through the symbol `num`.",
            prediction: "Which eight memory bytes change when 10 is added to `num`?",
            module_name: "addGlobal.s",
            source: include_str!("../../../course-content/lecture4/addGlobal.s"),
        },
        Lesson {
            id: "addglobalbetter",
            title: "The same global, RIP-relative",
            lecture: 4,
            summary: "Repeat the global update with modern RIP-relative operands.",
            prediction: "Does `num(%rip)` read the address of `num`, or the value stored there?",
            module_name: "addGlobalBetter.s",
            source: include_str!("../../../course-content/lecture4/addGlobalBetter.s"),
        },
        Lesson {
            id: "addgloballea",
            title: "lea means address, parentheses mean value",
            lecture: 4,
            summary: "Put &num in a register, then dereference it for an update.",
            prediction: "Why does `lea num(%rip),%rbx` not produce 200?",
            module_name: "addGlobalLea.s",
            source: include_str!("../../../course-content/lecture4/addGlobalLea.s"),
        },
        Lesson {
            id: "addarray1",
            title: "An array begins at its first element",
            lecture: 4,
            summary: "Treat a four-quad array label as the address of element zero.",
            prediction: "Which of the four quadwords will `(%rbx)` select?",
            module_name: "addArray1.s",
            source: include_str!("../../../course-content/lecture4/addArray1.s"),
        },
        Lesson {
            id: "addarray2",
            title: "Array indexing by moving a pointer",
            lecture: 4,
            summary: "Advance a quadword pointer by eight bytes before dereferencing.",
            prediction: "After adding 8 to &num, which array element does %rbx address?",
            module_name: "addArray2.s",
            source: include_str!("../../../course-content/lecture4/addArray2.s"),
        },
        Lesson {
            id: "addarray3",
            title: "Base + index × scale",
            lecture: 4,
            summary: "Select a quadword with the full scaled-index addressing mode.",
            prediction: "Evaluate &num + 1×8 before stepping the memory instruction.",
            module_name: "addArray3.s",
            source: include_str!("../../../course-content/lecture4/addArray3.s"),
        },
        Lesson {
            id: "addarray4",
            title: ".long changes width and stride",
            lecture: 4,
            summary: "Use 32-bit operations and notice that an eight-byte stride skips an element.",
            prediction: "With four-byte elements, where does &num + 8 land?",
            module_name: "addArray4.s",
            source: include_str!("../../../course-content/lecture4/addArray4.s"),
        },
        Lesson {
            id: "cmp1",
            title: "cmp sets flags; jge reads them",
            lecture: 4,
            summary: "Compare 20 with 10 and inspect the signed greater-or-equal predicate.",
            prediction: "For `cmp %rbx,%rcx`, is the conceptual subtraction 20−10 or 10−20?",
            module_name: "cmp1.s",
            source: include_str!("../../../course-content/lecture4/cmp1.s"),
        },
        Lesson {
            id: "sumloop",
            title: "Count upward to 55",
            lecture: 4,
            summary: "Build a loop from add, cmp, and a signed conditional jump.",
            prediction: "When %rcx reaches 10, will `jle` take the branch one more time?",
            module_name: "sumLoop.s",
            source: include_str!("../../../course-content/lecture4/sumLoop.s"),
        },
        Lesson {
            id: "sumloopb",
            title: "Count downward to 55",
            lecture: 4,
            summary: "Compute the same sum while the counter approaches zero.",
            prediction: "What flag state finally makes `jg loopStart` fall through?",
            module_name: "sumLoopB.s",
            source: include_str!("../../../course-content/lecture4/sumLoopB.s"),
        },
        Lesson {
            id: "hello",
            title: "Hello, bytes: the write syscall",
            lecture: 4,
            summary: "Lay out an asciz string and pass fd, address, and byte count to write.",
            prediction: "Does `hellolen` include the newline and the terminating NUL byte?",
            module_name: "hello.s",
            source: include_str!("../../../course-content/lecture4/hello.s"),
        },
        Lesson {
            id: "echo",
            title: "Read blocks, then returns a byte count",
            lecture: 5,
            summary: "Read one terminal line into .bss and write exactly the returned bytes back.",
            prediction: "What can the read syscall put in %rax before the program knows how many bytes to write?",
            module_name: "echo.s",
            source: include_str!("../../../course-content/lecture5/echo.s"),
        },
        Lesson {
            id: "helloret",
            title: "A syscall has a return value too",
            lecture: 5,
            summary: "Use write's returned byte count as the eventual process status.",
            prediction: "How many bytes does write return when the string includes newline and NUL?",
            module_name: "helloRet.s",
            source: include_str!("../../../course-content/lecture5/helloRet.s"),
        },
        Lesson {
            id: "routine",
            title: "The hard-coded way back from a routine",
            lecture: 5,
            summary: "Jump into a routine and discover why a fixed return label does not compose.",
            prediction: "Where is the return destination recorded when both transfers are plain jmp instructions?",
            module_name: "routine.s",
            source: include_str!("../../../course-content/lecture5/routine.s"),
        },
        Lesson {
            id: "fun1",
            title: "call, ret, and a clobbered argument",
            lecture: 5,
            summary: "Watch call push a return address and see a function overwrite %rdi.",
            prediction: "Why does the second call double 40 instead of the original 20?",
            module_name: "fun1.s",
            source: include_str!("../../../course-content/lecture5/fun1.s"),
        },
        Lesson {
            id: "fun2",
            title: "Preserving the value changes the answer",
            lecture: 5,
            summary: "Keep %rdi intact while computing through %r9, then compare with fun1.",
            prediction: "The second call still receives %rdi=20; what result will it return?",
            module_name: "fun2.s",
            source: include_str!("../../../course-content/lecture5/fun2.s"),
        },
        Lesson {
            id: "funstack",
            title: "A frame with two local quadwords",
            lecture: 5,
            summary: "Build and tear down a real frame containing a return address, saved %rbp, and locals.",
            prediction: "After subtracting 16 from %rsp, where are -8(%rbp) and -16(%rbp)?",
            module_name: "funStack.s",
            source: include_str!("../../../course-content/lecture5/funStack.s"),
        },
        Lesson {
            id: "readwrite",
            title: "Link readInt and writeInt",
            lecture: 6,
            summary: "Cross module boundaries, parse a line of digits, and print the integer again.",
            prediction: "Which module owns the input buffer, and which return value crosses each call?",
            module_name: "readWriteTest.s",
            source: include_str!("../../../course-content/lecture6/readWriteTest.s"),
        },
        Lesson {
            id: "facttrace",
            title: "Factorial frame tracing lab",
            lecture: 6,
            summary: "Hold the input at 5 and concentrate on recursive frame growth and unwinding.",
            prediction: "How many active fact frames appear before the base case returns 1?",
            module_name: "factTrace.s",
            source: include_str!("../../../course-content/lecture6/factTrace.s"),
        },
        Lesson {
            id: "fact",
            title: "Recursive factorial across three modules",
            lecture: 6,
            summary: "Read an integer, grow one frame per recursive call, multiply while returning, and print the result.",
            prediction: "For input 5, which value must each frame preserve at -8(%rbp)?",
            module_name: "fact.s",
            source: include_str!("../../../course-content/lecture6/fact.s"),
        },
        Lesson {
            id: "sumlooparray",
            title: "Sum an array, then call writeInt",
            lecture: 6,
            summary: "Combine scaled indexing, a loop, and a helper function from another module.",
            prediction: "What value reaches writeInt after the four quadwords are accumulated?",
            module_name: "sumLoopArray.s",
            source: include_str!("../../../course-content/lecture6/sumLoopArray.s"),
        },
        Lesson {
            id: "sum5",
            title: "Assignment 2 · Read, store, sum, print",
            lecture: 0,
            summary: "Exercise the whole assignment rubric with five interactive inputs, an array, procedures, and output.",
            prediction: "After the fifth input, where are the five values stored and which procedure produces 150?",
            module_name: "sum5.s",
            source: include_str!("../../../course-content/assignment2/sum5.s"),
        },
        Lesson {
            id: "guidelocallabels",
            title: "Guide · Numeric local labels",
            lecture: 0,
            summary: "Reuse `1:` in several routines and follow `1f` forward to the nearest definition below.",
            prediction: "Inside biglabel1, which `1:` does `jmp 1f` reach?",
            module_name: "localLabels.s",
            source: include_str!("../../../course-content/guide/localLabels.s"),
        },
        Lesson {
            id: "guidesign",
            title: "Guide exercise 1 · Sign of a number",
            lecture: 0,
            summary: "Compare against zero and choose one of three exits with two conditional jumps.",
            prediction: "With %rax = -42, which jumps are skipped and what will `echo $?` print?",
            module_name: "signOfNumber.s",
            source: include_str!("../../../course-content/guide/signOfNumber.s"),
        },
        Lesson {
            id: "guidesum",
            title: "Guide exercise 2 · Sum from 1 to 10",
            lecture: 0,
            summary: "Count with inc, test with cmp/jg, and accumulate in %rax.",
            prediction: "How many times does `add %rcx,%rax` run before `jg done` is taken?",
            module_name: "sumOneToTen.s",
            source: include_str!("../../../course-content/guide/sumOneToTen.s"),
        },
        Lesson {
            id: "guidemax",
            title: "Guide exercise 3 · Max of an array",
            lecture: 0,
            summary: "Compute an element count with `(. - arr) / 8` and scan with scaled-index addressing.",
            prediction: "What value does the assembler give `arrlen`, and which element becomes the max?",
            module_name: "maxOfArray.s",
            source: include_str!("../../../course-content/guide/maxOfArray.s"),
        },
        Lesson {
            id: "guideabs",
            title: "Guide exercise 4 · Absolute value function",
            lecture: 0,
            summary: "Pass an argument in %rdi, negate when negative, and return in %rax.",
            prediction: "Which instruction runs only when the argument is negative?",
            module_name: "absoluteValue.s",
            source: include_str!("../../../course-content/guide/absoluteValue.s"),
        },
        Lesson {
            id: "guidelocals",
            title: "Guide exercise 5 · Function with local variables",
            lecture: 0,
            summary: "Store two squared intermediates in a 16-byte frame and subtract them.",
            prediction: "Which addresses do -8(%rbp) and -16(%rbp) name inside diffsq?",
            module_name: "localVariables.s",
            source: include_str!("../../../course-content/guide/localVariables.s"),
        },
        Lesson {
            id: "guidefact",
            title: "Guide exercise 6 · Recursive factorial",
            lecture: 0,
            summary: "Save n in a local slot across each recursive call and multiply on the way back.",
            prediction: "How many frames are live at the deepest point of fact(5)?",
            module_name: "recursiveFactorial.s",
            source: include_str!("../../../course-content/guide/recursiveFactorial.s"),
        },
        Lesson {
            id: "guidesumn",
            title: "Guide exercise 7 · Sum of 1 to N, with I/O",
            lecture: 0,
            summary: "Read N with readInt, count down with test/jz, and print with writeInt.",
            prediction: "For N = 10, what does `jz sum_done` check on each pass?",
            module_name: "sumToN.s",
            source: include_str!("../../../course-content/guide/sumToN.s"),
        },
        Lesson {
            id: "signeddivide",
            title: "Guide detour · Signed division with cqo and idivq",
            lecture: 0,
            summary: "Sign-extend a negative dividend into %rdx before dividing it as a signed value.",
            prediction: "What would %rdx hold if the program used `xor %rdx,%rdx` instead of cqo?",
            module_name: "signedDivide.s",
            source: include_str!("../../../course-content/guide/signedDivide.s"),
        },
        Lesson {
            id: "missingimmediate",
            title: "Repair a missing immediate marker",
            lecture: 0,
            summary: "Use the assembler diagnostic to distinguish a literal from a memory address.",
            prediction: "Why does `60` mean something different from `$60` in an operand?",
            module_name: "missingImmediate.s",
            source: include_str!("../../../course-content/practice/missingImmediate.s"),
        },
        Lesson {
            id: "reversedoperands",
            title: "Repair reversed operands",
            lecture: 0,
            summary: "Use a destination diagnostic to restore AT&T source,destination order.",
            prediction: "Which operand must be writable in `mov source,destination`?",
            module_name: "reversedOperands.s",
            source: include_str!("../../../course-content/practice/reversedOperands.s"),
        },
        Lesson {
            id: "wrongwidth",
            title: "Find an accidental wide read",
            lecture: 0,
            summary: "See a quadword load consume two neighboring .long values.",
            prediction: "What eight bytes will `movq score(%rip),%rdi` combine?",
            module_name: "wrongWidth.s",
            source: include_str!("../../../course-content/practice/wrongWidth.s"),
        },
        Lesson {
            id: "callersavedloop",
            title: "Keep a loop counter across a call",
            lecture: 0,
            summary: "Repair a loop that trusts a caller-saved register across writeInt.",
            prediction: "May writeInt change %rcx before returning to the loop?",
            module_name: "callerSavedLoop.s",
            source: include_str!("../../../course-content/practice/callerSavedLoop.s"),
        },
        Lesson {
            id: "misalignedcall",
            title: "Repair call-site alignment",
            lecture: 0,
            summary: "Preserve a stack value while restoring 16-byte alignment before call.",
            prediction: "After one push, what is %rsp modulo 16 immediately before call?",
            module_name: "misalignedCall.s",
            source: include_str!("../../../course-content/practice/misalignedCall.s"),
        },
    ];
    &LESSONS
}

pub fn support_modules(id: &str) -> &'static [LessonModule] {
    static READ_INT: LessonModule = LessonModule {
        module_name: "readInt.s",
        source: include_str!("../../../course-content/lecture6/readInt.s"),
    };
    static WRITE_INT: LessonModule = LessonModule {
        module_name: "writeInt.s",
        source: include_str!("../../../course-content/lecture6/writeInt.s"),
    };
    static IO_HELPERS: [LessonModule; 2] = [READ_INT, WRITE_INT];
    static WRITE_HELPER: [LessonModule; 1] = [WRITE_INT];
    match id {
        "readwrite" | "fact" | "sum5" | "guidesumn" => &IO_HELPERS,
        "sumlooparray" | "callersavedloop" | "misalignedcall" => &WRITE_HELPER,
        _ => &[],
    }
}

pub fn assignment(id: &str) -> Option<&'static Assignment> {
    static SUM5: Assignment = Assignment {
        goal: "Run the complete minimal Assignment 2 workflow using five separate terminal lines.",
        success: "The array contains 10, 20, 30, 40, and 50; stdout ends in 150 and a newline; the process exits normally.",
        requirements: &[
            "Read user input through a called procedure.",
            "Store values in a five-element quadword array.",
            "Compute the result in a procedure reached with call.",
            "Write the result to standard output.",
        ],
        sample_inputs: &["10", "20", "30", "40", "50"],
        expected_output: "Enter a number: Enter a number: Enter a number: Enter a number: Enter a number: 150\\n",
    };
    match id {
        "sum5" => Some(&SUM5),
        _ => None,
    }
}

/// Complete programs from the CS201 assembly guide (`assemblyGuide.org`).
pub const GUIDE_LESSONS: &[&str] = &[
    "guidelocallabels",
    "guidesign",
    "guidesum",
    "guidemax",
    "guideabs",
    "guidelocals",
    "guidefact",
    "guidesumn",
    "signeddivide",
];

pub fn collection(id: &str, lecture: u8) -> &'static str {
    if assignment(id).is_some() {
        "assignment"
    } else if GUIDE_LESSONS.contains(&id) {
        "guide"
    } else if challenge(id).is_some() {
        "practice"
    } else if lecture > 0 {
        "lecture"
    } else {
        unreachable!("every unnumbered lesson belongs to an explicit collection")
    }
}

pub fn challenge(id: &str) -> Option<&'static Challenge> {
    static MISSING_IMMEDIATE: Challenge = Challenge {
        goal: "Repair the operand, assemble the program, and exit cleanly.",
        success: "The source assembles and the process exits with shell status 0.",
        hints: &[
            "The diagnostic distinguishes a literal value from an address.",
            "Immediate operands begin with `$` in GNU/AT&T syntax.",
            "Change `60` to `$60`; do not change the syscall.",
        ],
    };
    static REVERSED_OPERANDS: Challenge = Challenge {
        goal: "Repair the mov instruction without changing the intended value or register.",
        success: "%rax receives 60 and the process exits with shell status 0.",
        hints: &[
            "An immediate value cannot be a destination.",
            "AT&T syntax is `instruction source,destination`.",
            "The repaired instruction is `mov $60,%rax`.",
        ],
    };
    static WRONG_WIDTH: Challenge = Challenge {
        goal: "Load only score, not the adjacent .long, and leave the result in %rdi.",
        success: "The full 64-bit value in %rdi is 255 when the process exits.",
        hints: &[
            "`.long` reserves four bytes, while the current mov reads eight.",
            "A 32-bit register destination also clears the upper half of its 64-bit register.",
            "Use `movl score(%rip),%edi`.",
        ],
    };
    static CALLER_SAVED_LOOP: Challenge = Challenge {
        goal: "Make the program print 321 while keeping writeInt unchanged.",
        success: "stdout is exactly `321` and the process exits normally.",
        hints: &[
            "%rcx is caller-saved, so writeInt is allowed to overwrite it.",
            "The helper documents that it preserves %rbx.",
            "Use %rbx consistently as the loop counter instead of %rcx.",
        ],
    };
    static MISALIGNED_CALL: Challenge = Challenge {
        goal: "Keep 42 on the stack, print it, and make the call with %rsp divisible by 16.",
        success: "stdout is `42`, the process exits normally, and every call receipt says aligned.",
        hints: &[
            "The initial %rsp is aligned; pushing one quadword changes its remainder to 8.",
            "Reserve one additional eight-byte padding slot before call, then release it afterward.",
            "Insert `sub $8,%rsp` before call and `add $8,%rsp` after it.",
        ],
    };
    match id {
        "missingimmediate" => Some(&MISSING_IMMEDIATE),
        "reversedoperands" => Some(&REVERSED_OPERANDS),
        "wrongwidth" => Some(&WRONG_WIDTH),
        "callersavedloop" => Some(&CALLER_SAVED_LOOP),
        "misalignedcall" => Some(&MISALIGNED_CALL),
        _ => None,
    }
}

pub fn lesson_views() -> Vec<LessonView<'static>> {
    lessons()
        .iter()
        .map(|lesson| LessonView {
            lesson,
            support_modules: support_modules(lesson.id),
            challenge: challenge(lesson.id),
            collection: collection(lesson.id, lesson.lecture),
            assignment: assignment(lesson.id),
        })
        .collect()
}

pub fn lesson(id: &str) -> Option<&'static Lesson> {
    lessons().iter().find(|lesson| lesson.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lesson_ids_are_unique_and_sources_have_start() {
        for (index, lesson) in lessons().iter().enumerate() {
            assert!(lesson.source.contains("_start:"));
            assert!(!lesson.source.trim().is_empty());
            assert!(lessons()[..index].iter().all(|prior| prior.id != lesson.id));
            for module in support_modules(lesson.id) {
                assert!(!module.source.trim().is_empty());
                assert!(!module.module_name.is_empty());
            }
            assert_eq!(
                challenge(lesson.id).is_some(),
                collection(lesson.id, lesson.lecture) == "practice"
            );
            assert_eq!(
                assignment(lesson.id).is_some(),
                collection(lesson.id, lesson.lecture) == "assignment"
            );
        }
    }
}
