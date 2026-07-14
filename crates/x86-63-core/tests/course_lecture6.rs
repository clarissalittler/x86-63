use x86_63_core::{Command, MachineStatus, Session, SourceModule, StepEvent};

fn lesson_session(id: &str) -> Session {
    let lesson = x86_63_course::lesson(id).unwrap();
    let mut modules = vec![SourceModule::new(lesson.module_name, lesson.source)];
    modules.extend(
        x86_63_course::support_modules(id)
            .iter()
            .map(|module| SourceModule::new(module.module_name, module.source)),
    );
    Session::from_modules(modules).unwrap_or_else(|error| panic!("{id}: {error}"))
}

fn register(view: &x86_63_core::MachineView, name: &str) -> u64 {
    view.registers
        .iter()
        .find(|register| register.name == name)
        .unwrap()
        .unsigned
        .parse()
        .unwrap()
}

#[test]
fn maintained_lecture_6_programs_link_and_run() {
    for (id, input, output, shell_status) in [
        ("readwrite", Some("123"), "123", 0),
        ("fact", Some("5"), "Enter a number: 120", 0),
        ("sumlooparray", None, "10", 0),
        ("facttrace", None, "", 120),
    ] {
        let mut session = lesson_session(id);
        if let Some(input) = input {
            session.execute(Command::SubmitInput {
                text: input.to_string(),
            });
        }
        let result = session.execute(Command::Continue { max_steps: 2_000 });
        assert!(result.diagnostics.is_empty(), "{id}");
        assert!(matches!(
            result.view.status,
            MachineStatus::Exited {
                shell_status: actual,
                ..
            } if actual == shell_status
        ));
        assert_eq!(result.view.io.stdout_escaped, output, "{id}");
        assert_eq!(result.view.stack.rsp, result.view.stack.top, "{id}");
        assert!(result.view.stack.frames.is_empty(), "{id}");
    }
}

#[test]
fn next_crosses_modules_and_preserves_call_alignment() {
    let mut session = lesson_session("readwrite");
    session.execute(Command::SubmitInput {
        text: "456".to_string(),
    });
    let result = session.execute(Command::Next);
    assert!(result.steps_executed > 10);
    assert_eq!(
        result.view.next_instruction.as_ref().unwrap().module,
        "readWriteTest.s"
    );
    assert_eq!(result.view.next_instruction.as_ref().unwrap().line, 8);
    assert_eq!(register(&result.view, "rax"), 456);
    assert!(result.view.stack.slots.is_empty());
    assert!(result.events.iter().any(|event| matches!(
        event,
        StepEvent::Call {
            target,
            aligned_before: true,
            ..
        } if target == "readInt"
    )));
    assert!(result.events.iter().any(|event| matches!(
        event,
        StepEvent::Call { target, .. } if target == "parseInt"
    )));
}

#[test]
fn recursive_frame_chain_is_visible_and_reversible() {
    let mut session = lesson_session("facttrace");
    for _ in 0..100 {
        if session.view().stack.frames.len() == 5 {
            break;
        }
        session.execute(Command::Step);
    }
    let deepest = session.view();
    assert_eq!(deepest.stack.frames.len(), 5);
    assert!(
        deepest
            .stack
            .frames
            .iter()
            .all(|frame| frame.function.as_deref() == Some("fact") && frame.aligned_at_call)
    );
    assert_eq!(register(&deepest, "rdi"), 1);

    let reversed = session.execute(Command::Back);
    assert_eq!(reversed.view.stack.frames.len(), 4);
    assert_eq!(register(&reversed.view, "rbp"), 0x0000_7fff_ffff_df90);
}

#[test]
fn write_int_handles_zero_negative_and_long_min() {
    let helper = x86_63_course::support_modules("sumlooparray")[0];
    for (instruction, expected) in [
        ("mov $0,%rdi", "0"),
        ("mov $-42,%rdi", "-42"),
        ("movabsq $9223372036854775808,%rdi", "-9223372036854775808"),
    ] {
        let harness = format!(
            ".text\n.global _start\n.extern writeInt\n_start:\n {instruction}\n call writeInt\n mov $60,%rax\n xor %rdi,%rdi\n syscall\n"
        );
        let mut session = Session::from_modules(vec![
            SourceModule::new("harness.s", harness),
            SourceModule::new(helper.module_name, helper.source),
        ])
        .unwrap();
        let result = session.execute(Command::Continue { max_steps: 1_000 });
        assert_eq!(result.view.io.stdout_escaped, expected);
        assert!(matches!(result.view.status, MachineStatus::Exited { .. }));
    }
}
