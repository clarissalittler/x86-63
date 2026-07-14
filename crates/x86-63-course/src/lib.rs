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

#[derive(Serialize)]
pub struct LessonView<'a> {
    #[serde(flatten)]
    pub lesson: &'a Lesson,
    pub support_modules: &'static [LessonModule],
}

pub fn lessons() -> &'static [Lesson] {
    static LESSONS: [Lesson; 25] = [
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
        "readwrite" | "fact" => &IO_HELPERS,
        "sumlooparray" => &WRITE_HELPER,
        _ => &[],
    }
}

pub fn lesson_views() -> Vec<LessonView<'static>> {
    lessons()
        .iter()
        .map(|lesson| LessonView {
            lesson,
            support_modules: support_modules(lesson.id),
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
        }
    }
}
