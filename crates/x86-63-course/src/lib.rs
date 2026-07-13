use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Lesson {
    pub id: &'static str,
    pub title: &'static str,
    pub lecture: u8,
    pub summary: &'static str,
    pub prediction: &'static str,
    pub module_name: &'static str,
    pub source: &'static str,
}

pub fn lessons() -> &'static [Lesson] {
    static LESSONS: [Lesson; 4] = [
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
    ];
    &LESSONS
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
        }
    }
}
