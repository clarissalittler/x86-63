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

#[test]
fn maintained_lecture_5_examples_have_the_expected_outcomes() {
    for lesson in x86_63_course::lessons()
        .iter()
        .filter(|lesson| lesson.lecture == 5)
    {
        let mut session = session(lesson.id);
        if lesson.id == "echo" {
            session.execute(Command::SubmitInput {
                text: "CS201".to_string(),
            });
        }
        let result = session.execute(Command::Continue { max_steps: 200 });
        assert!(result.diagnostics.is_empty(), "{}", lesson.id);
        let expected_status = match lesson.id {
            "echo" => 0,
            "helloret" => 14,
            "routine" | "fun2" | "funstack" => 40,
            "fun1" => 80,
            unexpected => panic!("add an explicit expectation for {unexpected}"),
        };
        assert!(matches!(
            result.view.status,
            MachineStatus::Exited { shell_status, .. } if shell_status == expected_status
        ));
        assert_eq!(
            result.view.stack.rsp, result.view.stack.top,
            "{}",
            lesson.id
        );
        assert!(result.view.stack.slots.is_empty(), "{}", lesson.id);
        if lesson.id == "echo" {
            assert_eq!(result.view.io.stdin_escaped, "CS201\\n");
            assert_eq!(result.view.io.stdin_consumed, 6);
            assert_eq!(result.view.io.stdout_escaped, "CS201\\n");
            assert_eq!(&result.view.memory.bytes[..6], b"CS201\n");
        }
    }
}

#[test]
fn read_blocks_without_input_and_reverse_restores_the_buffer_and_queue() {
    let mut blocked = session("echo");
    let result = blocked.execute(Command::Continue { max_steps: 100 });
    assert!(matches!(
        result.view.status,
        MachineStatus::WaitingForInput { count: 128, .. }
    ));
    assert_eq!(result.steps_executed, 4);
    assert_eq!(result.view.history_depth, 4);
    assert_eq!(
        result.view.next_text.as_deref().map(str::trim),
        Some("syscall")
    );
    assert!(
        result
            .events
            .iter()
            .any(|event| matches!(event, StepEvent::InputRequested { count: 128, .. }))
    );

    blocked.execute(Command::SubmitInput {
        text: "hello".to_string(),
    });
    let read = blocked.execute(Command::Step);
    assert_eq!(&read.view.memory.bytes[..6], b"hello\n");
    assert_eq!(read.view.io.stdin_consumed, 6);
    assert_eq!(register(&read.view, "rax"), "6");

    let reversed = blocked.execute(Command::Back);
    assert_eq!(&reversed.view.memory.bytes[..6], &[0; 6]);
    assert_eq!(reversed.view.io.stdin_consumed, 0);
    assert_eq!(register(&reversed.view, "rax"), "0");
    assert_eq!(
        reversed.view.next_text.as_deref().map(str::trim),
        Some("syscall")
    );
}

#[test]
fn next_steps_over_a_call_while_step_reveals_the_return_address() {
    let mut next_session = session("fun1");
    next_session.execute(Command::Step);
    let next = next_session.execute(Command::Next);
    assert_eq!(next.steps_executed, 4);
    assert_eq!(
        next.view.next_text.as_deref().map(str::trim),
        Some("mov %rax,%rdi")
    );
    assert_eq!(register(&next.view, "rax"), "40");
    assert!(next.view.stack.slots.is_empty());

    let mut step_session = session("funstack");
    step_session.execute(Command::Step);
    let call = step_session.execute(Command::Step);
    assert_eq!(call.view.stack.slots.len(), 1);
    assert!(
        call.view.stack.slots[0]
            .label
            .as_deref()
            .is_some_and(|label| label.contains("return to"))
    );
    assert!(
        call.events
            .iter()
            .any(|event| matches!(event, StepEvent::Call { target, .. } if target == "fun"))
    );

    step_session.execute(Command::Step); // push %rbp
    step_session.execute(Command::Step); // mov %rsp,%rbp
    let frame = step_session.execute(Command::Step); // sub $16,%rsp
    assert_eq!(frame.view.stack.slots.len(), 4);
    assert!(
        frame
            .view
            .stack
            .slots
            .iter()
            .any(|slot| slot.offset_from_rbp == Some(-16))
    );
    assert!(
        frame
            .view
            .stack
            .slots
            .iter()
            .any(|slot| slot.label.as_deref() == Some("saved caller %rbp"))
    );
}
