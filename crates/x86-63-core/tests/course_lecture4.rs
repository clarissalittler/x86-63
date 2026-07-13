use x86_63_core::{Command, MachineStatus, MachineView, Session, SourceModule, StepEvent};

fn session(id: &str) -> Session {
    let lesson = x86_63_course::lesson(id).unwrap();
    Session::from_modules(vec![SourceModule::new(lesson.module_name, lesson.source)])
        .unwrap_or_else(|error| panic!("{id} did not assemble: {error}"))
}

fn register<'a>(view: &'a MachineView, name: &str) -> &'a str {
    &view
        .registers
        .iter()
        .find(|register| register.name == name)
        .unwrap()
        .unsigned
}

fn little_u64(view: &MachineView, offset: usize) -> u64 {
    u64::from_le_bytes(view.memory.bytes[offset..offset + 8].try_into().unwrap())
}

fn little_u32(view: &MachineView, offset: usize) -> u32 {
    u32::from_le_bytes(view.memory.bytes[offset..offset + 4].try_into().unwrap())
}

#[test]
fn maintained_lecture_4_examples_have_the_expected_outcomes() {
    for lesson in x86_63_course::lessons()
        .iter()
        .filter(|lesson| lesson.lecture == 4)
    {
        let mut session = session(lesson.id);
        let result = session.execute(Command::Continue { max_steps: 100 });
        assert!(result.diagnostics.is_empty(), "{}", lesson.id);

        let expected_status = match lesson.id {
            "addglobal" | "addglobalbetter" | "addgloballea" | "addarray1" | "addarray2" => 210,
            "addarray3" => 54,
            "addarray4" => 160,
            "cmp1" => 255,
            "sumloop" | "sumloopb" => 55,
            "hello" => 0,
            unexpected => panic!("add an explicit expectation for {unexpected}"),
        };
        assert!(matches!(
            result.view.status,
            MachineStatus::Exited { shell_status, .. } if shell_status == expected_status
        ));

        match lesson.id {
            "addglobal" | "addglobalbetter" | "addgloballea" | "addarray1" => {
                assert_eq!(little_u64(&result.view, 0), 210)
            }
            "addarray2" => assert_eq!(little_u64(&result.view, 8), 210),
            "addarray3" => {
                assert_eq!(little_u64(&result.view, 8), 310);
                assert_eq!(register(&result.view, "rdi"), "310");
            }
            "addarray4" => assert_eq!(little_u32(&result.view, 8), 160),
            "sumloop" | "sumloopb" => assert_eq!(register(&result.view, "rbx"), "55"),
            "hello" => {
                assert_eq!(result.view.io.stdout_bytes, b"Hello world!\n\0");
                assert_eq!(result.view.io.stdout_escaped, "Hello world!\\n\\0");
            }
            "cmp1" => assert_eq!(register(&result.view, "rdi"), u64::MAX.to_string()),
            _ => unreachable!(),
        }
    }
}

#[test]
fn reverse_step_restores_memory_and_output() {
    let mut memory_session = session("addgloballea");
    memory_session.execute(Command::Step);
    let write = memory_session.execute(Command::Step);
    assert_eq!(little_u64(&write.view, 0), 210);
    assert!(write.events.iter().any(|event| matches!(
        event,
        StepEvent::MemoryWrite { symbol, .. } if symbol.as_deref() == Some("num")
    )));
    let reversed = memory_session.execute(Command::Back);
    assert_eq!(little_u64(&reversed.view, 0), 200);

    let mut output_session = session("hello");
    output_session.execute(Command::Continue { max_steps: 100 });
    assert!(!output_session.view().io.stdout_bytes.is_empty());
    for _ in 0..4 {
        output_session.execute(Command::Back);
    }
    assert!(output_session.view().io.stdout_bytes.is_empty());
    assert_eq!(register(&output_session.view(), "rax"), "1");
}

#[test]
fn branch_events_explain_the_concrete_predicate() {
    let mut session = session("cmp1");
    for _ in 0..4 {
        session.execute(Command::Step);
    }
    let branch = session.execute(Command::Step);
    assert!(branch.events.iter().any(|event| matches!(
        event,
        StepEvent::Branch {
            condition,
            target,
            taken: true,
            ..
        } if condition == "jge" && target == "greater"
    )));
    assert!(branch.explanation.contains("SF=0 equals OF=0"));

    let reversed = session.execute(Command::Back);
    assert_eq!(
        reversed.view.next_text.as_deref().map(str::trim),
        Some("jge greater")
    );
}
